// @LITE_DESC Spring Boot REST controller with CRUD endpoints, validation, error handling, and DTO pattern
// @LITE_SCENE rest-api
// @LITE_TAGS java,spring,rest,controller,api

import org.springframework.boot.SpringApplication;
import org.springframework.boot.autoconfigure.SpringBootApplication;
import org.springframework.http.ResponseEntity;
import org.springframework.validation.annotation.Validated;
import org.springframework.web.bind.annotation.*;
import org.springframework.web.servlet.support.ServletUriComponentsBuilder;

import jakarta.validation.Valid;
import jakarta.validation.constraints.*;
import java.net.URI;
import java.time.LocalDateTime;
import java.util.*;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicLong;

/**
 * Main Spring Boot application.
 */
@SpringBootApplication
public class RestApiApplication {
    public static void main(String[] args) {
        SpringApplication.run(RestApiApplication.class, args);
    }
}

/**
 * User DTO for request/response.
 */
class UserDTO {
    private Long id;

    @NotBlank(message = "Name is required")
    @Size(min = 2, max = 100, message = "Name must be between 2 and 100 characters")
    private String name;

    @NotBlank(message = "Email is required")
    @Email(message = "Email must be valid")
    private String email;

    @Size(min = 10, max = 20, message = "Phone must be between 10 and 20 characters")
    private String phone;

    private LocalDateTime createdAt;
    private LocalDateTime updatedAt;

    public UserDTO() {}

    public UserDTO(String name, String email, String phone) {
        this.name = name;
        this.email = email;
        this.phone = phone;
        this.createdAt = LocalDateTime.now();
        this.updatedAt = LocalDateTime.now();
    }

    // Getters and setters
    public Long getId() { return id; }
    public void setId(Long id) { this.id = id; }

    public String getName() { return name; }
    public void setName(String name) { this.name = name; }

    public String getEmail() { return email; }
    public void setEmail(String email) { this.email = email; }

    public String getPhone() { return phone; }
    public void setPhone(String phone) { this.phone = phone; }

    public LocalDateTime getCreatedAt() { return createdAt; }
    public void setCreatedAt(LocalDateTime createdAt) { this.createdAt = createdAt; }

    public LocalDateTime getUpdatedAt() { return updatedAt; }
    public void setUpdatedAt(LocalDateTime updatedAt) { this.updatedAt = updatedAt; }
}

/**
 * Response wrapper for consistent API responses.
 */
class ApiResponse<T> {
    private boolean success;
    private String message;
    private T data;
    private LocalDateTime timestamp;

    public ApiResponse() {
        this.timestamp = LocalDateTime.now();
    }

    public ApiResponse(boolean success, String message, T data) {
        this.success = success;
        this.message = message;
        this.data = data;
        this.timestamp = LocalDateTime.now();
    }

    public static <T> ApiResponse<T> success(T data) {
        return new ApiResponse<>(true, "Success", data);
    }

    public static <T> ApiResponse<T> success(String message, T data) {
        return new ApiResponse<>(true, message, data);
    }

    public static <T> ApiResponse<T> error(String message) {
        return new ApiResponse<>(false, message, null);
    }

    // Getters
    public boolean isSuccess() { return success; }
    public String getMessage() { return message; }
    public T getData() { return data; }
    public LocalDateTime getTimestamp() { return timestamp; }
}

/**
 * Custom exception for resource not found.
 */
class ResourceNotFoundException extends RuntimeException {
    public ResourceNotFoundException(String message) {
        super(message);
    }
}

/**
 * Global exception handler.
 */
@RestControllerAdvice
class GlobalExceptionHandler {

    @ExceptionHandler(ResourceNotFoundException.class)
    public ResponseEntity<ApiResponse<Object>> handleResourceNotFound(ResourceNotFoundException ex) {
        return ResponseEntity
            .status(404)
            .body(ApiResponse.error(ex.getMessage()));
    }

    @ExceptionHandler(MethodArgumentNotValidException.class)
    public ResponseEntity<ApiResponse<Object>> handleValidationException(
            MethodArgumentNotValidException ex) {

        Map<String, String> errors = new HashMap<>();
        ex.getBindingResult().getFieldErrors().forEach(error ->
            errors.put(error.getField(), error.getDefaultMessage())
        );

        return ResponseEntity
            .status(400)
            .body(ApiResponse.error("Validation failed: " + errors));
    }

    @ExceptionHandler(Exception.class)
    public ResponseEntity<ApiResponse<Object>> handleGenericException(Exception ex) {
        return ResponseEntity
            .status(500)
            .body(ApiResponse.error("Internal server error: " + ex.getMessage()));
    }
}

/**
 * User repository (in-memory for demo).
 */
@Repository
class UserRepository {
    private final Map<Long, UserDTO> users = new ConcurrentHashMap<>();
    private final AtomicLong idGenerator = new AtomicLong(1);

    public List<UserDTO> findAll() {
        return new ArrayList<>(users.values());
    }

    public Optional<UserDTO> findById(Long id) {
        return Optional.ofNullable(users.get(id));
    }

    public UserDTO save(UserDTO user) {
        if (user.getId() == null) {
            user.setId(idGenerator.getAndIncrement());
            user.setCreatedAt(LocalDateTime.now());
        }
        user.setUpdatedAt(LocalDateTime.now());
        users.put(user.getId(), user);
        return user;
    }

    public void deleteById(Long id) {
        users.remove(id);
    }

    public boolean existsById(Long id) {
        return users.containsKey(id);
    }

    public List<UserDTO> findByNameContaining(String name) {
        return users.values().stream()
            .filter(user -> user.getName().toLowerCase().contains(name.toLowerCase()))
            .toList();
    }
}

/**
 * REST Controller for user management.
 */
@RestController
@RequestMapping("/api/users")
@Validated
class UserController {

    private final UserRepository repository;

    public UserController(UserRepository repository) {
        this.repository = repository;
    }

    /**
     * Get all users with optional filtering.
     * GET /api/users?name=John
     */
    @GetMapping
    public ResponseEntity<ApiResponse<List<UserDTO>>> getAllUsers(
            @RequestParam(required = false) String name) {

        List<UserDTO> users;
        if (name != null && !name.isEmpty()) {
            users = repository.findByNameContaining(name);
        } else {
            users = repository.findAll();
        }

        return ResponseEntity.ok(ApiResponse.success(users));
    }

    /**
     * Get user by ID.
     * GET /api/users/{id}
     */
    @GetMapping("/{id}")
    public ResponseEntity<ApiResponse<UserDTO>> getUserById(@PathVariable Long id) {
        UserDTO user = repository.findById(id)
            .orElseThrow(() -> new ResourceNotFoundException("User not found with id: " + id));

        return ResponseEntity.ok(ApiResponse.success(user));
    }

    /**
     * Create new user.
     * POST /api/users
     */
    @PostMapping
    public ResponseEntity<ApiResponse<UserDTO>> createUser(
            @Valid @RequestBody UserDTO userDTO) {

        // Check if email already exists
        boolean emailExists = repository.findAll().stream()
            .anyMatch(user -> user.getEmail().equalsIgnoreCase(userDTO.getEmail()));

        if (emailExists) {
            return ResponseEntity
                .badRequest()
                .body(ApiResponse.error("Email already exists"));
        }

        UserDTO savedUser = repository.save(userDTO);

        URI location = ServletUriComponentsBuilder
            .fromCurrentRequest()
            .path("/{id}")
            .buildAndExpand(savedUser.getId())
            .toUri();

        return ResponseEntity
            .created(location)
            .body(ApiResponse.success("User created successfully", savedUser));
    }

    /**
     * Update existing user.
     * PUT /api/users/{id}
     */
    @PutMapping("/{id}")
    public ResponseEntity<ApiResponse<UserDTO>> updateUser(
            @PathVariable Long id,
            @Valid @RequestBody UserDTO userDTO) {

        if (!repository.existsById(id)) {
            throw new ResourceNotFoundException("User not found with id: " + id);
        }

        userDTO.setId(id);
        UserDTO updatedUser = repository.save(userDTO);

        return ResponseEntity.ok(
            ApiResponse.success("User updated successfully", updatedUser)
        );
    }

    /**
     * Partially update user.
     * PATCH /api/users/{id}
     */
    @PatchMapping("/{id}")
    public ResponseEntity<ApiResponse<UserDTO>> patchUser(
            @PathVariable Long id,
            @RequestBody Map<String, Object> updates) {

        UserDTO existingUser = repository.findById(id)
            .orElseThrow(() -> new ResourceNotFoundException("User not found with id: " + id));

        // Apply partial updates
        if (updates.containsKey("name")) {
            existingUser.setName((String) updates.get("name"));
        }
        if (updates.containsKey("email")) {
            existingUser.setEmail((String) updates.get("email"));
        }
        if (updates.containsKey("phone")) {
            existingUser.setPhone((String) updates.get("phone"));
        }

        UserDTO updatedUser = repository.save(existingUser);

        return ResponseEntity.ok(
            ApiResponse.success("User patched successfully", updatedUser)
        );
    }

    /**
     * Delete user.
     * DELETE /api/users/{id}
     */
    @DeleteMapping("/{id}")
    public ResponseEntity<ApiResponse<Void>> deleteUser(@PathVariable Long id) {
        if (!repository.existsById(id)) {
            throw new ResourceNotFoundException("User not found with id: " + id);
        }

        repository.deleteById(id);

        return ResponseEntity.noContent().build();
    }

    /**
     * Get user statistics.
     * GET /api/users/stats
     */
    @GetMapping("/stats")
    public ResponseEntity<ApiResponse<Map<String, Object>>> getStats() {
        List<UserDTO> allUsers = repository.findAll();

        Map<String, Object> stats = new HashMap<>();
        stats.put("totalUsers", allUsers.size());
        stats.put("usersCreatedToday", allUsers.stream()
            .filter(user -> user.getCreatedAt().toLocalDate().equals(LocalDateTime.now().toLocalDate()))
            .count());
        stats.put("usersUpdatedThisWeek", allUsers.stream()
            .filter(user -> user.getUpdatedAt().isAfter(LocalDateTime.now().minusWeeks(1)))
            .count());

        return ResponseEntity.ok(ApiResponse.success(stats));
    }
}
