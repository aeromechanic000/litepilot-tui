// @LITE_DESC Fetch API wrapper with HTTP methods, error handling, and retry logic
// @LITE_SCENE A comprehensive Fetch API utility with async/await patterns and error handling
// @LITE_TAGS javascript,fetch,api,http,async,request

class ApiClient {
  constructor(baseUrl = '', options = {}) {
    this.baseUrl = baseUrl;
    this.defaultOptions = {
      headers: {
        'Content-Type': 'application/json',
      },
      ...options
    };
  }

  async request(endpoint, options = {}) {
    const url = `${this.baseUrl}${endpoint}`;
    const config = { ...this.defaultOptions, ...options };

    try {
      const response = await fetch(url, config);

      if (!response.ok) {
        throw new ApiError(response.status, response.statusText);
      }

      const contentType = response.headers.get('content-type');
      if (contentType && contentType.includes('application/json')) {
        return await response.json();
      }

      return await response.text();

    } catch (error) {
      if (error instanceof ApiError) {
        throw error;
      }
      throw new ApiError(0, error.message);
    }
  }

  async get(endpoint, options = {}) {
    return this.request(endpoint, { ...options, method: 'GET' });
  }

  async post(endpoint, data, options = {}) {
    return this.request(endpoint, {
      ...options,
      method: 'POST',
      body: JSON.stringify(data)
    });
  }

  async put(endpoint, data, options = {}) {
    return this.request(endpoint, {
      ...options,
      method: 'PUT',
      body: JSON.stringify(data)
    });
  }

  async delete(endpoint, options = {}) {
    return this.request(endpoint, { ...options, method: 'DELETE' });
  }

  async patch(endpoint, data, options = {}) {
    return this.request(endpoint, {
      ...options,
      method: 'PATCH',
      body: JSON.stringify(data)
    });
  }
}

class ApiError extends Error {
  constructor(status, message) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
  }
}

// Retry utility with exponential backoff
async function fetchWithRetry(url, options = {}, maxRetries = 3) {
  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      const response = await fetch(url, options);

      if (!response.ok) {
        throw new ApiError(response.status, response.statusText);
      }

      return response;

    } catch (error) {
      if (attempt === maxRetries) {
        throw error;
      }

      const delay = Math.pow(2, attempt) * 1000; // Exponential backoff
      console.log(`Attempt ${attempt} failed. Retrying in ${delay}ms...`);
      await sleep(delay);
    }
  }
}

function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

// Request timeout wrapper
async function fetchWithTimeout(url, options = {}, timeout = 5000) {
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), timeout);

  try {
    const response = await fetch(url, {
      ...options,
      signal: controller.signal
    });
    clearTimeout(timeoutId);
    return response;

  } catch (error) {
    clearTimeout(timeoutId);
    if (error.name === 'AbortError') {
      throw new ApiError(0, 'Request timeout');
    }
    throw error;
  }
}

// Concurrent requests with Promise.all
async function fetchMultiple(urls, options = {}) {
  try {
    const responses = await Promise.all(
      urls.map(url => fetch(url, options))
    );

    const data = await Promise.all(
      responses.map(response => {
        if (!response.ok) {
          throw new ApiError(response.status, response.statusText);
        }
        return response.json();
      })
    );

    return data;

  } catch (error) {
    console.error('Error in concurrent requests:', error);
    throw error;
  }
}

// Sequential requests with results accumulation
async function fetchSequential(requests) {
  const results = [];

  for (const request of requests) {
    try {
      const response = await fetch(request.url, request.options);

      if (!response.ok) {
        throw new ApiError(response.status, response.statusText);
      }

      const data = await response.json();
      results.push({ success: true, data });

    } catch (error) {
      results.push({ success: false, error: error.message });
    }
  }

  return results;
}

// Usage examples
const api = new ApiClient('https://api.example.com');

async function examples() {
  try {
    // GET request
    const users = await api.get('/users');
    console.log('Users:', users);

    // POST request
    const newUser = await api.post('/users', {
      name: 'John Doe',
      email: 'john@example.com'
    });
    console.log('New user:', newUser);

    // PUT request
    const updatedUser = await api.put(`/users/${newUser.id}`, {
      name: 'Jane Doe'
    });
    console.log('Updated user:', updatedUser);

    // DELETE request
    await api.delete(`/users/${newUser.id}`);
    console.log('User deleted');

    // Retry example
    const data = await fetchWithRetry('https://api.example.com/data');
    console.log('Data with retry:', await data.json());

    // Timeout example
    const response = await fetchWithTimeout(
      'https://api.example.com/quick',
      {},
      2000
    );
    console.log('Quick response:', await response.json());

    // Concurrent requests
    const multipleData = await fetchMultiple([
      'https://api.example.com/users',
      'https://api.example.com/posts',
      'https://api.example.com/comments'
    ]);
    console.log('Multiple data:', multipleData);

    // Sequential requests
    const sequentialResults = await fetchSequential([
      { url: 'https://api.example.com/step1', options: { method: 'POST' } },
      { url: 'https://api.example.com/step2', options: { method: 'POST' } },
      { url: 'https://api.example.com/step3', options: { method: 'POST' } }
    ]);
    console.log('Sequential results:', sequentialResults);

  } catch (error) {
    console.error('API error:', error.message, 'Status:', error.status);
  }
}

// Export for use in other modules
if (typeof module !== 'undefined' && module.exports) {
  module.exports = {
    ApiClient,
    ApiError,
    fetchWithRetry,
    fetchWithTimeout,
    fetchMultiple,
    fetchSequential
  };
}
