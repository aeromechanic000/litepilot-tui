# @LITE_DESC: Pytest test file with fixtures, parametrize, mocking, async tests, tmp_path usage
# @LITE_SCENE: A comprehensive pytest template showcasing fixtures, parameterization, mocking, async testing, and temporary file handling
# @LITE_TAGS: python, pytest, testing, unit, mock

import pytest
from pathlib import Path
from unittest.mock import Mock, patch, MagicMock, AsyncMock
from datetime import datetime
import asyncio
import json
from typing import Dict, List

# Example classes to test
class UserService:
    """Service class for user management"""

    def __init__(self, db_client):
        self.db = db_client

    def get_user(self, user_id: int) -> Dict:
        """Get user by ID"""
        return self.db.query(f"SELECT * FROM users WHERE id = {user_id}")

    def create_user(self, username: str, email: str) -> Dict:
        """Create a new user"""
        if not username or not email:
            raise ValueError("Username and email are required")

        user_data = {
            'username': username,
            'email': email,
            'created_at': datetime.now().isoformat()
        }
        return self.db.insert('users', user_data)

    def delete_user(self, user_id: int) -> bool:
        """Delete user by ID"""
        return self.db.delete('users', user_id)

class AsyncDataService:
    """Async service for data operations"""

    def __init__(self, api_client):
        self.api = api_client

    async def fetch_data(self, endpoint: str) -> Dict:
        """Fetch data from API endpoint"""
        response = await self.api.get(endpoint)
        return response.json()

    async def process_multiple(self, endpoints: List[str]) -> List[Dict]:
        """Process multiple endpoints concurrently"""
        tasks = [self.fetch_data(endpoint) for endpoint in endpoints]
        return await asyncio.gather(*tasks)

# Fixtures
@pytest.fixture
def sample_user_data():
    """Fixture providing sample user data"""
    return {
        'id': 1,
        'username': 'testuser',
        'email': 'test@example.com',
        'created_at': '2024-01-01T00:00:00'
    }

@pytest.fixture
def mock_db_client():
    """Fixture providing a mocked database client"""
    mock_client = Mock()
    mock_client.query.return_value = {'id': 1, 'username': 'testuser'}
    mock_client.insert.return_value = {'id': 1, 'username': 'testuser', 'email': 'test@example.com'}
    mock_client.delete.return_value = True
    return mock_client

@pytest.fixture
def user_service(mock_db_client):
    """Fixture providing a UserService instance with mocked dependencies"""
    return UserService(mock_db_client)

@pytest.fixture
def temp_config_file(tmp_path):
    """Fixture creating a temporary configuration file"""
    config_data = {
        'api_key': 'test_key_123',
        'timeout': 30,
        'retries': 3
    }
    config_file = tmp_path / "config.json"
    config_file.write_text(json.dumps(config_data))
    return config_file

@pytest.fixture
def sample_csv_file(tmp_path):
    """Fixture creating a temporary CSV file with sample data"""
    csv_content = """name,age,city
Alice,25,NYC
Bob,30,LA
Charlie,35,Chicago"""
    csv_file = tmp_path / "sample.csv"
    csv_file.write_text(csv_content)
    return csv_file

@pytest.fixture
async def mock_api_client():
    """Fixture providing a mocked async API client"""
    mock_client = AsyncMock()
    mock_client.get.return_value.json.return_value = {'status': 'success', 'data': [1, 2, 3]}
    return mock_client

@pytest.fixture
def async_data_service(mock_api_client):
    """Fixture providing an AsyncDataService instance with mocked dependencies"""
    return AsyncDataService(mock_api_client)

# Parametrized tests
@pytest.mark.parametrize("username, email, expected_result", [
    ("testuser", "test@example.com", True),
    ("user123", "user123@example.com", True),
    ("admin", "admin@example.com", True),
])
def test_create_user_valid_inputs(user_service, username, email, expected_result):
    """Test user creation with various valid inputs"""
    result = user_service.create_user(username, email)
    assert result is not None
    assert result['username'] == username

@pytest.mark.parametrize("invalid_input, error_message", [
    ("", "test@example.com", "Username and email are required"),
    ("testuser", "", "Username and email are required"),
    (None, "test@example.com", "Username and email are required"),
    ("testuser", None, "Username and email are required"),
])
def test_create_user_invalid_inputs(user_service, invalid_input, error_message):
    """Test user creation with invalid inputs"""
    with pytest.raises(ValueError, match=error_message):
        user_service.create_user(invalid_input, invalid_input if invalid_input else "")

@pytest.mark.parametrize("endpoint, mock_response", [
    ("/api/users", {'users': [{'id': 1}, {'id': 2}]}),
    ("/api/posts", {'posts': [{'id': 1}, {'id': 2}, {'id': 3}]}),
    ("/api/comments", {'comments': []}),
])
async def test_fetch_data_different_endpoints(async_data_service, endpoint, mock_response):
    """Test fetching data from different endpoints"""
    async_data_service.api.get.return_value.json.return_value = mock_response

    result = await async_data_service.fetch_data(endpoint)
    assert result == mock_response

# Tests using fixtures
def test_get_user(user_service):
    """Test getting a user by ID"""
    result = user_service.get_user(1)
    assert result is not None
    assert 'username' in result
    user_service.db.query.assert_called_once()

def test_delete_user(user_service):
    """Test deleting a user"""
    result = user_service.delete_user(1)
    assert result is True
    user_service.db.delete.assert_called_once_with('users', 1)

def test_temp_config_file_reading(temp_config_file):
    """Test reading from temporary configuration file"""
    with open(temp_config_file, 'r') as f:
        config = json.load(f)

    assert config['api_key'] == 'test_key_123'
    assert config['timeout'] == 30
    assert config['retries'] == 3

def test_temp_csv_file_reading(sample_csv_file):
    """Test reading from temporary CSV file"""
    content = sample_csv_file.read_text()
    lines = content.strip().split('\n')

    assert len(lines) == 4  # header + 3 data rows
    assert 'name,age,city' in lines[0]
    assert 'Alice,25,NYC' in content

def test_tmp_path_usage(tmp_path):
    """Test creating and reading temporary files using tmp_path fixture"""
    # Create a temporary file
    temp_file = tmp_path / "test.txt"
    temp_file.write_text("Hello, World!")

    # Read it back
    content = temp_file.read_text()
    assert content == "Hello, World!"

    # Verify file exists
    assert temp_file.exists()
    assert temp_file.is_file()

# Mocking tests
@patch('builtins.open', new_callable=MagicMock)
def test_file_operations_with_mock(mock_open):
    """Test file operations using mocked open function"""
    mock_open.return_value.__enter__.return_value.read.return_value = "mocked content"

    with open('test.txt', 'r') as f:
        content = f.read()

    assert content == "mocked content"
    mock_open.assert_called_once_with('test.txt', 'r')

def test_user_service_with_mocked_db():
    """Test UserService with completely mocked database"""
    mock_db = Mock()
    mock_db.query.return_value = {'id': 999, 'username': 'mocked_user'}

    service = UserService(mock_db)
    result = service.get_user(999)

    assert result['username'] == 'mocked_user'
    mock_db.query.assert_called_once()

@patch('requests.get')
def test_external_api_call(mock_get):
    """Test external API call with mocked requests"""
    mock_response = Mock()
    mock_response.status_code = 200
    mock_response.json.return_value = {'data': 'test'}
    mock_get.return_value = mock_response

    response = mock_get('https://api.example.com/data')
    assert response.status_code == 200
    assert response.json() == {'data': 'test'}

# Async tests
@pytest.mark.asyncio
async def test_async_fetch_data(async_data_service):
    """Test async data fetching"""
    result = await async_data_service.fetch_data('/api/test')
    assert result is not None
    assert 'status' in result
    async_data_service.api.get.assert_called_once_with('/api/test')

@pytest.mark.asyncio
async def test_async_process_multiple(async_data_service):
    """Test processing multiple endpoints concurrently"""
    endpoints = ['/api/users', '/api/posts', '/api/comments']

    results = await async_data_service.process_multiple(endpoints)

    assert len(results) == 3
    assert all(isinstance(result, dict) for result in results)
    assert async_data_service.api.get.call_count == 3

@pytest.mark.asyncio
async def test_async_error_handling():
    """Test async error handling"""
    async def failing_function():
        raise ValueError("Async error occurred")

    with pytest.raises(ValueError, match="Async error occurred"):
        await failing_function()

# Test class
class TestUserService:
    """Test class for UserService with setup and teardown"""

    @pytest.fixture(autouse=True)
    def setup(self, mock_db_client):
        """Setup method called before each test"""
        self.service = UserService(mock_db_client)
        self.mock_db = mock_db_client

    def test_initialization(self):
        """Test service initialization"""
        assert self.service.db is not None
        assert isinstance(self.service, UserService)

    def test_get_user_success(self):
        """Test successful user retrieval"""
        self.mock_db.query.return_value = {'id': 1, 'username': 'testuser'}
        result = self.service.get_user(1)
        assert result['username'] == 'testuser'

    def test_create_user_success(self):
        """Test successful user creation"""
        result = self.service.create_user('newuser', 'new@example.com')
        assert result['username'] == 'newuser'
        self.mock_db.insert.assert_called_once()

# Pytest hooks and configuration
def pytest_configure(config):
    """Configure pytest with custom markers"""
    config.addinivalue_line(
        "markers", "slow: marks tests as slow (deselect with '-m \"not slow\"')"
    )
    config.addinivalue_line(
        "markers", "integration: marks tests as integration tests"
    )

# Example test with custom marker
@pytest.mark.slow
def test_slow_operation():
    """Example of a test marked as slow"""
    import time
    time.sleep(0.1)
    assert True

# Run example
if __name__ == '__main__':
    pytest.main([__file__, '-v', '--tb=short'])
