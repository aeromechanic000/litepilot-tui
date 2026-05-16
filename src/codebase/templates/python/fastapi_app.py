# @LITE_DESC: FastAPI app with Pydantic models, dependency injection, middleware, error handling, async DB
# @LITE_SCENE: A production-ready FastAPI application with Pydantic validation, dependency injection, custom middleware, comprehensive error handling, and async database operations
# @LITE_TAGS: python, fastapi, api, async, pydantic

from fastapi import FastAPI, Depends, HTTPException, status, Request, Response
from fastapi.middleware.cors import CORSMiddleware
from fastapi.security import HTTPBearer, HTTPAuthorizationCredentials
from fastapi.responses import JSONResponse
from pydantic import BaseModel, Field, validator, EmailStr
from typing import Optional, List, Dict, Any
from datetime import datetime, timedelta
from enum import Enum
import asyncio
import logging
from contextlib import asynccontextmanager
import uvicorn

# Setup logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

# Pydantic Models
class UserRole(str, Enum):
    ADMIN = "admin"
    USER = "user"
    GUEST = "guest"

class UserBase(BaseModel):
    username: str = Field(..., min_length=3, max_length=50)
    email: EmailStr
    full_name: Optional[str] = None

    @validator('username')
    def username_alphanumeric(cls, v):
        if not v.isalnum():
            raise ValueError('Username must be alphanumeric')
        return v

class UserCreate(UserBase):
    password: str = Field(..., min_length=8)
    role: UserRole = UserRole.USER

class UserUpdate(BaseModel):
    email: Optional[EmailStr] = None
    full_name: Optional[str] = None
    role: Optional[UserRole] = None

class UserResponse(UserBase):
    id: int
    role: UserRole
    created_at: datetime
    is_active: bool

    class Config:
        orm_mode = True

class PostBase(BaseModel):
    title: str = Field(..., min_length=1, max_length=200)
    content: str = Field(..., min_length=1)
    published: bool = False

class PostCreate(PostBase):
    tags: List[str] = []

class PostUpdate(BaseModel):
    title: Optional[str] = None
    content: Optional[str] = None
    published: Optional[bool] = None
    tags: Optional[List[str]] = None

class PostResponse(PostBase):
    id: int
    author_id: int
    created_at: datetime
    updated_at: datetime
    tags: List[str]

    class Config:
        orm_mode = True

class ErrorResponse(BaseModel):
    error: str
    message: str
    details: Optional[Dict[str, Any]] = None

# Mock Database (In production, use actual async DB like asyncpg/aiosqlite)
class MockDatabase:
    def __init__(self):
        self.users: Dict[int, Dict] = {}
        self.posts: Dict[int, Dict] = {}
        self.user_counter = 1
        self.post_counter = 1
        self._initialized = False

    async def initialize(self):
        """Initialize database with sample data"""
        if not self._initialized:
            await self.create_user(
                username="admin",
                email="admin@example.com",
                password="admin123",
                role=UserRole.ADMIN
            )
            self._initialized = True
            logger.info("Database initialized")

    async def create_user(self, username: str, email: str, password: str, role: UserRole) -> Dict:
        user_id = self.user_counter
        self.user_counter += 1

        user = {
            "id": user_id,
            "username": username,
            "email": email,
            "password": password,  # In production, hash this!
            "role": role,
            "created_at": datetime.now(),
            "is_active": True
        }
        self.users[user_id] = user
        return user

    async def get_user(self, user_id: int) -> Optional[Dict]:
        return self.users.get(user_id)

    async def get_user_by_username(self, username: str) -> Optional[Dict]:
        for user in self.users.values():
            if user["username"] == username:
                return user
        return None

    async def update_user(self, user_id: int, **kwargs) -> Optional[Dict]:
        if user_id in self.users:
            self.users[user_id].update(kwargs)
            return self.users[user_id]
        return None

    async def delete_user(self, user_id: int) -> bool:
        if user_id in self.users:
            del self.users[user_id]
            return True
        return False

    async def create_post(self, title: str, content: str, author_id: int,
                         published: bool = False, tags: List[str] = None) -> Dict:
        post_id = self.post_counter
        self.post_counter += 1

        post = {
            "id": post_id,
            "title": title,
            "content": content,
            "author_id": author_id,
            "published": published,
            "tags": tags or [],
            "created_at": datetime.now(),
            "updated_at": datetime.now()
        }
        self.posts[post_id] = post
        return post

    async def get_post(self, post_id: int) -> Optional[Dict]:
        return self.posts.get(post_id)

    async def get_posts_by_author(self, author_id: int) -> List[Dict]:
        return [post for post in self.posts.values() if post["author_id"] == author_id]

# Global database instance
db = MockDatabase()

# Lifespan context manager
@asynccontextmanager
async def lifespan(app: FastAPI):
    # Startup
    await db.initialize()
    logger.info("Application started")
    yield
    # Shutdown
    logger.info("Application stopped")

# Initialize FastAPI app
app = FastAPI(
    title="FastAPI Application",
    description="A production-ready FastAPI application with comprehensive features",
    version="1.0.0",
    lifespan=lifespan
)

# CORS Middleware
app.add_middleware(
    CORSMiddleware,
    allow_origins=["http://localhost:3000", "http://localhost:8080"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Custom Middleware
@app.middleware("http")
async def logging_middleware(request: Request, call_next):
    """Log all requests and responses"""
    start_time = datetime.now()

    logger.info(f"Request: {request.method} {request.url}")

    response = await call_next(request)

    process_time = (datetime.now() - start_time).total_seconds()
    response.headers["X-Process-Time"] = str(process_time)

    logger.info(f"Response: {response.status_code} - {process_time:.4f}s")
    return response

@app.middleware("http")
async def error_handling_middleware(request: Request, call_next):
    """Global error handling middleware"""
    try:
        return await call_next(request)
    except Exception as e:
        logger.error(f"Unhandled exception: {str(e)}")
        return JSONResponse(
            status_code=500,
            content={"error": "Internal Server Error", "message": str(e)}
        )

# Dependencies
security = HTTPBearer()

async def get_current_user(
    credentials: HTTPAuthorizationCredentials = Depends(security)
) -> Dict:
    """Get current user from JWT token (simplified)"""
    token = credentials.credentials
    # In production, validate JWT token here
    # For demo, return admin user
    user = await db.get_user_by_username("admin")
    if not user:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Invalid authentication credentials"
        )
    return user

async def get_db():
    """Get database instance"""
    return db

class PaginationParams:
    def __init__(self, skip: int = 0, limit: int = 100):
        self.skip = skip
        self.limit = limit

# Exception handlers
class AppException(Exception):
    def __init__(self, status_code: int, message: str, details: Dict = None):
        self.status_code = status_code
        self.message = message
        self.details = details

@app.exception_handler(AppException)
async def app_exception_handler(request: Request, exc: AppException):
    return JSONResponse(
        status_code=exc.status_code,
        content={
            "error": "Application Error",
            "message": exc.message,
            "details": exc.details
        }
    )

# API Routes
@app.get("/", tags=["Root"])
async def root():
    """Root endpoint"""
    return {
        "message": "Welcome to FastAPI Application",
        "version": "1.0.0",
        "docs": "/docs",
        "redoc": "/redoc"
    }

@app.get("/health", tags=["Health"])
async def health_check():
    """Health check endpoint"""
    return {
        "status": "healthy",
        "timestamp": datetime.now().isoformat()
    }

# User endpoints
@app.post("/api/users", response_model=UserResponse, status_code=status.HTTP_201_CREATED, tags=["Users"])
async def create_user(user: UserCreate, database: MockDatabase = Depends(get_db)):
    """Create a new user"""
    existing_user = await database.get_user_by_username(user.username)
    if existing_user:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail="Username already exists"
        )

    db_user = await database.create_user(
        username=user.username,
        email=user.email,
        password=user.password,
        role=user.role
    )

    return UserResponse(**db_user)

@app.get("/api/users/{user_id}", response_model=UserResponse, tags=["Users"])
async def get_user(user_id: int, database: MockDatabase = Depends(get_db)):
    """Get user by ID"""
    user = await database.get_user(user_id)
    if not user:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="User not found"
        )

    return UserResponse(**user)

@app.put("/api/users/{user_id}", response_model=UserResponse, tags=["Users"])
async def update_user(
    user_id: int,
    user_update: UserUpdate,
    database: MockDatabase = Depends(get_db),
    current_user: Dict = Depends(get_current_user)
):
    """Update user"""
    if current_user["role"] != UserRole.ADMIN and current_user["id"] != user_id:
        raise HTTPException(
            status_code=status.HTTP_403_FORBIDDEN,
            detail="Not authorized to update this user"
        )

    update_data = user_update.dict(exclude_unset=True)
    updated_user = await database.update_user(user_id, **update_data)

    if not updated_user:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="User not found"
        )

    return UserResponse(**updated_user)

@app.delete("/api/users/{user_id}", status_code=status.HTTP_204_NO_CONTENT, tags=["Users"])
async def delete_user(
    user_id: int,
    database: MockDatabase = Depends(get_db),
    current_user: Dict = Depends(get_current_user)
):
    """Delete user"""
    if current_user["role"] != UserRole.ADMIN:
        raise HTTPException(
            status_code=status.HTTP_403_FORBIDDEN,
            detail="Only admins can delete users"
        )

    success = await database.delete_user(user_id)
    if not success:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="User not found"
        )

    return None

# Post endpoints
@app.post("/api/posts", response_model=PostResponse, status_code=status.HTTP_201_CREATED, tags=["Posts"])
async def create_post(
    post: PostCreate,
    database: MockDatabase = Depends(get_db),
    current_user: Dict = Depends(get_current_user)
):
    """Create a new post"""
    db_post = await database.create_post(
        title=post.title,
        content=post.content,
        author_id=current_user["id"],
        published=post.published,
        tags=post.tags
    )

    return PostResponse(**db_post)

@app.get("/api/posts/{post_id}", response_model=PostResponse, tags=["Posts"])
async def get_post(post_id: int, database: MockDatabase = Depends(get_db)):
    """Get post by ID"""
    post = await database.get_post(post_id)
    if not post:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="Post not found"
        )

    return PostResponse(**post)

@app.get("/api/posts", response_model=List[PostResponse], tags=["Posts"])
async def list_posts(
    skip: int = 0,
    limit: int = 100,
    database: MockDatabase = Depends(get_db)
):
    """List all posts with pagination"""
    posts = list(database.posts.values())[skip:skip+limit]
    return [PostResponse(**post) for post in posts]

@app.get("/api/users/{user_id}/posts", response_model=List[PostResponse], tags=["Posts"])
async def get_user_posts(user_id: int, database: MockDatabase = Depends(get_db)):
    """Get all posts by a specific user"""
    posts = await database.get_posts_by_author(user_id)
    return [PostResponse(**post) for post in posts]

# Run the application
if __name__ == "__main__":
    uvicorn.run(
        "fastapi_app:app",
        host="0.0.0.0",
        port=8000,
        reload=True,
        log_level="info"
    )
