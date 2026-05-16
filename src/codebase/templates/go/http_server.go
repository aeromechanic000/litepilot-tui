// @LITE_DESC Go HTTP server with mux routing, middleware stack, JSON handling, graceful shutdown, and structured logging
// @LITE_SCENE Production-ready web server demonstrating REST endpoints, request validation, middleware pipeline, and clean shutdown
// @LITE_TAGS go, http, server, web, api

package main

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"os"
	"os/signal"
	"sync"
	"syscall"
	"time"

	"github.com/gorilla/mux"
)

// AppState holds shared application state
type AppState struct {
	startupTime time.Time
	users       *UserStore
	mu          sync.RWMutex
}

// UserStore manages in-memory user storage
type UserStore struct {
	users map[string]*User
	mu    sync.RWMutex
}

// User represents a user entity
type User struct {
	ID    string `json:"id"`
	Name  string `json:"name"`
	Email string `json:"email"`
}

// CreateUserRequest represents user creation payload
type CreateUserRequest struct {
	Name  string `json:"name"`
	Email string `json:"email"`
}

// ErrorResponse represents error responses
type ErrorResponse struct {
	Error   string `json:"error"`
	Code    int    `json:"code"`
	Path    string `json:"path,omitempty"`
	Method  string `json:"method,omitempty"`
}

// Response wrapper for consistent responses
type Response struct {
	Success bool        `json:"success"`
	Data    interface{} `json:"data,omitempty"`
	Error   *ErrorResponse `json:"error,omitempty"`
}

// Middleware chains
type Middleware func(http.Handler) http.Handler

func chain(middlewares ...Middleware) Middleware {
	return func(next http.Handler) http.Handler {
		for i := len(middlewares) - 1; i >= 0; i-- {
			next = middlewares[i](next)
		}
		return next
	}
}

// Logging middleware
func loggingMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		start := time.Now()

		// Wrap response writer to capture status code
		wrapped := &responseWrapper{ResponseWriter: w, status: http.StatusOK}

		next.ServeHTTP(wrapped, r)

		duration := time.Since(start)
		log.Printf("%s %s %d %v", r.Method, r.URL.Path, wrapped.status, duration)
	})
}

// CORS middleware
func corsMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Access-Control-Allow-Origin", "*")
		w.Header().Set("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
		w.Header().Set("Access-Control-Allow-Headers", "Content-Type, Authorization")

		if r.Method == "OPTIONS" {
			w.WriteHeader(http.StatusOK)
			return
		}

		next.ServeHTTP(w, r)
	})
}

// Response wrapper to capture status codes
type responseWrapper struct {
	http.ResponseWriter
	status int
}

func (w *responseWrapper) WriteHeader(status int) {
	w.status = status
	w.ResponseWriter.WriteHeader(status)
}

// JSON helper
func writeJSON(w http.ResponseWriter, status int, data interface{}) error {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	return json.NewEncoder(w).Encode(data)
}

func writeError(w http.ResponseWriter, status int, message string, r *http.Request) error {
	resp := Response{
		Success: false,
		Error: &ErrorResponse{
			Error:  message,
			Code:   status,
			Path:   r.URL.Path,
			Method: r.Method,
		},
	}
	return writeJSON(w, status, resp)
}

// Handler functions
func (app *AppState) healthCheck(w http.ResponseWriter, r *http.Request) {
	writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
}

func (app *AppState) getInfo(w http.ResponseWriter, r *http.Request) {
	uptime := time.Since(app.startupTime)
	info := map[string]interface{}{
		"uptime":   uptime.String(),
		"started":  app.startupTime.Format(time.RFC3339),
		"version":  "1.0.0",
	}
	writeJSON(w, http.StatusOK, Response{Success: true, Data: info})
}

func (app *AppState) listUsers(w http.ResponseWriter, r *http.Request) {
	app.users.mu.RLock()
	defer app.users.mu.RUnlock()

	users := make([]*User, 0, len(app.users.users))
	for _, user := range app.users.users {
		users = append(users, user)
	}

	writeJSON(w, http.StatusOK, Response{Success: true, Data: users})
}

func (app *AppState) getUser(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	id := vars["id"]

	app.users.mu.RLock()
	user, exists := app.users.users[id]
	app.users.mu.RUnlock()

	if !exists {
		writeError(w, http.StatusNotFound, "User not found", r)
		return
	}

	writeJSON(w, http.StatusOK, Response{Success: true, Data: user})
}

func (app *AppState) createUser(w http.ResponseWriter, r *http.Request) {
	var req CreateUserRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeError(w, http.StatusBadRequest, "Invalid request body", r)
		return
	}

	if req.Name == "" || req.Email == "" {
		writeError(w, http.StatusBadRequest, "Name and email are required", r)
		return
	}

	user := &User{
		ID:    generateID(),
		Name:  req.Name,
		Email: req.Email,
	}

	app.users.mu.Lock()
	app.users.users[user.ID] = user
	app.users.mu.Unlock()

	writeJSON(w, http.StatusCreated, Response{Success: true, Data: user})
}

func generateID() string {
	return fmt.Sprintf("user-%d", time.Now().UnixNano())
}

func main() {
	// Initialize app state
	app := &AppState{
		startupTime: time.Now(),
		users: &UserStore{
			users: make(map[string]*User),
		},
	}

	// Add sample users
	app.users.users["user-1"] = &User{ID: "user-1", Name: "Alice", Email: "alice@example.com"}
	app.users.users["user-2"] = &User{ID: "user-2", Name: "Bob", Email: "bob@example.com"}

	// Create router
	r := mux.NewRouter()

	// Apply middleware
	middlewareChain := chain(
		corsMiddleware,
		loggingMiddleware,
	)

	// Register routes
	r.HandleFunc("/health", app.healthCheck).Methods("GET")
	r.HandleFunc("/info", app.getInfo).Methods("GET")
	r.HandleFunc("/users", app.listUsers).Methods("GET")
	r.HandleFunc("/users", app.createUser).Methods("POST")
	r.HandleFunc("/users/{id}", app.getUser).Methods("GET")

	// Create server
	srv := &http.Server{
		Addr:         ":3000",
		Handler:      middlewareChain(r),
		ReadTimeout:  15 * time.Second,
		WriteTimeout: 15 * time.Second,
		IdleTimeout:  60 * time.Second,
	}

	// Start server in goroutine
	go func() {
		log.Printf("Server starting on http://localhost%s", srv.Addr)
		if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			log.Fatalf("Server failed to start: %v", err)
		}
	}()

	// Graceful shutdown
	gracefulShutdown(srv)
}

func gracefulShutdown(srv *http.Server) {
	quit := make(chan os.Signal, 1)
	signal.Notify(quit, syscall.SIGINT, syscall.SIGTERM)
	<-quit

	log.Println("Shutdown signal received, shutting down gracefully...")

	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	if err := srv.Shutdown(ctx); err != nil {
		log.Printf("Server forced to shutdown: %v", err)
		os.Exit(1)
	}

	log.Println("Server shutdown complete")
}
