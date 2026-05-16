// @LITE_DESC Express server in TypeScript with typed middleware and routes
// @LITE_SCENE A strongly-typed Express.js server with TypeScript interfaces and type safety
// @LITE_TAGS typescript,express,server,typed,api

import express, {
  Request,
  Response,
  NextFunction,
  RequestHandler,
  ErrorRequestHandler
} from 'express';
import cors from 'cors';

// Type definitions
interface User {
  id: number;
  name: string;
  email: string;
  createdAt?: Date;
}

interface CreateUserRequest {
  name: string;
  email: string;
}

interface UpdateUserRequest {
  name?: string;
  email?: string;
}

interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
  message?: string;
}

interface ApiError extends Error {
  status?: number;
}

// Type guards
function isCreateUserRequest(body: any): body is CreateUserRequest {
  return (
    typeof body === 'object' &&
    body !== null &&
    'name' in body &&
    'email' in body &&
    typeof body.name === 'string' &&
    typeof body.email === 'string'
  );
}

// Express app setup
const app = express();
const PORT = process.env.PORT || 3000;

// Middleware
app.use(cors());
app.use(express.json());

// Type-safe logging middleware
const loggerMiddleware: RequestHandler = (req, res, next) => {
  const timestamp = new Date().toISOString();
  console.log(`[${timestamp}] ${req.method} ${req.path}`);
  next();
};

app.use(loggerMiddleware);

// Typed route handlers
const getUsersHandler: RequestHandler = (req: Request, res: Response) => {
  const users: User[] = [
    { id: 1, name: 'Alice', email: 'alice@example.com', createdAt: new Date() },
    { id: 2, name: 'Bob', email: 'bob@example.com', createdAt: new Date() }
  ];

  const response: ApiResponse<User[]> = {
    success: true,
    data: users
  };

  res.json(response);
};

const getUserByIdHandler: RequestHandler<{ id: string }> = (req, res) => {
  const userId = parseInt(req.params.id);

  const user: User = {
    id: userId,
    name: `User ${userId}`,
    email: `user${userId}@example.com`,
    createdAt: new Date()
  };

  const response: ApiResponse<User> = {
    success: true,
    data: user
  };

  res.json(response);
};

const createUserHandler: RequestHandler = async (req, res, next) => {
  try {
    if (!isCreateUserRequest(req.body)) {
      const error: ApiError = new Error('Invalid request body');
      error.status = 400;
      throw error;
    }

    const { name, email } = req.body;

    const newUser: User = {
      id: Date.now(),
      name,
      email,
      createdAt: new Date()
    };

    const response: ApiResponse<User> = {
      success: true,
      data: newUser,
      message: 'User created successfully'
    };

    res.status(201).json(response);
  } catch (error) {
    next(error);
  }
};

const updateUserHandler: RequestHandler<{ id: string }> = async (req, res, next) => {
  try {
    const userId = parseInt(req.params.id);
    const updates: UpdateUserRequest = req.body;

    const existingUser: User = {
      id: userId,
      name: `User ${userId}`,
      email: `user${userId}@example.com`
    };

    const updatedUser: User = {
      ...existingUser,
      ...updates
    };

    const response: ApiResponse<User> = {
      success: true,
      data: updatedUser,
      message: 'User updated successfully'
    };

    res.json(response);
  } catch (error) {
    next(error);
  }
};

const deleteUserHandler: RequestHandler<{ id: string }> = (req, res) => {
  const userId = parseInt(req.params.id);

  const response: ApiResponse<{ id: number }> = {
    success: true,
    data: { id: userId },
    message: 'User deleted successfully'
  };

  res.json(response);
};

// Register routes
app.get('/', (req: Request, res: Response) => {
  res.json({ message: 'Welcome to the TypeScript Express API' });
});

app.get('/api/users', getUsersHandler);
app.get('/api/users/:id', getUserByIdHandler);
app.post('/api/users', createUserHandler);
app.put('/api/users/:id', updateUserHandler);
app.delete('/api/users/:id', deleteUserHandler);

// 404 handler
app.use((req: Request, res: Response) => {
  res.status(404).json({
    success: false,
    error: 'Route not found'
  } as ApiResponse<never>);
});

// Type-safe error handler
const errorHandler: ErrorRequestHandler = (
  err: Error,
  req: Request,
  res: Response,
  next: NextFunction
) => {
  const apiError = err as ApiError;
  const status = apiError.status || 500;

  console.error('Error:', err);

  res.status(status).json({
    success: false,
    error: err.message || 'Internal server error'
  } as ApiResponse<never>);
};

app.use(errorHandler);

// Start server
app.listen(PORT, () => {
  console.log(`Server running on http://localhost:${PORT}`);
});

// Type exports for other modules
export { User, CreateUserRequest, UpdateUserRequest, ApiResponse };

// Example of typed middleware factory
function createAuthMiddleware(requiredRole: string): RequestHandler {
  return (req: Request, res: Response, next: NextFunction) => {
    const authHeader = req.headers.authorization;

    if (!authHeader) {
      const error: ApiError = new Error('Authorization header required');
      error.status = 401;
      return next(error);
    }

    // Simplified auth logic - in real app, verify token
    const token = authHeader.replace('Bearer ', '');

    if (!token) {
      const error: ApiError = new Error('Invalid token');
      error.status = 401;
      return next(error);
    }

    // Add user info to request (would need to extend Express Request type)
    // (req as any).user = { role: 'user' };

    next();
  };
}

// Example of typed utility functions
function successResponse<T>(data: T, message?: string): ApiResponse<T> {
  return {
    success: true,
    data,
    message
  };
}

function errorResponse(error: string): ApiResponse<never> {
  return {
    success: false,
    error
  };
}

export default app;
