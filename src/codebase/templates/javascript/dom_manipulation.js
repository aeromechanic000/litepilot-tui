// @LITE_DESC DOM manipulation examples with querySelector, events, and Fetch API
// @LITE_SCENE Browser-side DOM operations including element selection, event handling, and API calls
// @LITE_TAGS javascript,dom,browser,events,frontend

// Element Selection
document.addEventListener('DOMContentLoaded', () => {
  // Query selectors
  const titleElement = document.querySelector('h1');
  const buttonElements = document.querySelectorAll('.action-button');
  const firstForm = document.querySelector('#user-form');

  // Element manipulation
  if (titleElement) {
    titleElement.textContent = 'Updated Title';
    titleElement.style.color = '#333';
    titleElement.classList.add('highlight');
  }

  // Dynamic element creation
  const newParagraph = document.createElement('p');
  newParagraph.textContent = 'This is dynamically created content';
  newParagraph.classList.add('dynamic-content');
  document.body.appendChild(newParagraph);

  // Event Listeners
  buttonElements.forEach(button => {
    button.addEventListener('click', (event) => {
      console.log('Button clicked:', event.target.textContent);
      handleButtonClick(event);
    });
  });

  // Form handling
  if (firstForm) {
    firstForm.addEventListener('submit', handleFormSubmit);

    // Input validation
    const emailInput = firstForm.querySelector('input[type="email"]');
    if (emailInput) {
      emailInput.addEventListener('blur', validateEmail);
    }
  }

  // Class manipulation
  const toggleButton = document.querySelector('#toggle-class');
  if (toggleButton) {
    toggleButton.addEventListener('click', () => {
      titleElement?.classList.toggle('active');
    });
  }
});

// Event handler functions
function handleButtonClick(event) {
  const button = event.target;
  button.classList.add('clicked');

  setTimeout(() => {
    button.classList.remove('clicked');
  }, 200);
}

function handleFormSubmit(event) {
  event.preventDefault();

  const formData = new FormData(event.target);
  const data = Object.fromEntries(formData.entries());

  console.log('Form data:', data);

  // Send data to server
  submitFormData(data);
}

function validateEmail(event) {
  const email = event.target.value;
  const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

  if (!emailRegex.test(email)) {
    event.target.classList.add('invalid');
    showError('Please enter a valid email address');
  } else {
    event.target.classList.remove('invalid');
    hideError();
  }
}

// Helper functions
function showError(message) {
  const existingError = document.querySelector('.error-message');
  if (existingError) {
    existingError.remove();
  }

  const errorElement = document.createElement('div');
  errorElement.className = 'error-message';
  errorElement.textContent = message;
  errorElement.style.color = 'red';

  document.body.appendChild(errorElement);
}

function hideError() {
  const errorElement = document.querySelector('.error-message');
  if (errorElement) {
    errorElement.remove();
  }
}

// Fetch API calls
async function submitFormData(data) {
  try {
    const response = await fetch('/api/submit', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(data)
    });

    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }

    const result = await response.json();
    console.log('Success:', result);
    showSuccessMessage('Data submitted successfully!');

  } catch (error) {
    console.error('Error submitting form:', error);
    showError('Failed to submit form. Please try again.');
  }
}

async function loadUserData(userId) {
  try {
    const response = await fetch(`/api/users/${userId}`);

    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }

    const userData = await response.json();
    return userData;

  } catch (error) {
    console.error('Error loading user data:', error);
    throw error;
  }
}

function showSuccessMessage(message) {
  const successElement = document.createElement('div');
  successElement.className = 'success-message';
  successElement.textContent = message;
  successElement.style.cssText = 'color: green; padding: 10px; margin: 10px 0;';

  document.body.appendChild(successElement);

  setTimeout(() => {
    successElement.remove();
  }, 3000);
}

// Utility functions
function debounce(func, wait) {
  let timeout;
  return function executedFunction(...args) {
    const later = () => {
      clearTimeout(timeout);
      func(...args);
    };
    clearTimeout(timeout);
    timeout = setTimeout(later, wait);
  };
}

// Example of debounced search
const searchInput = document.querySelector('#search');
if (searchInput) {
  const debouncedSearch = debounce((query) => {
    console.log('Searching for:', query);
    performSearch(query);
  }, 300);

  searchInput.addEventListener('input', (event) => {
    debouncedSearch(event.target.value);
  });
}

async function performSearch(query) {
  try {
    const response = await fetch(`/api/search?q=${encodeURIComponent(query)}`);
    const results = await response.json();
    displaySearchResults(results);
  } catch (error) {
    console.error('Search error:', error);
  }
}

function displaySearchResults(results) {
  const resultsContainer = document.querySelector('#search-results');
  if (resultsContainer) {
    resultsContainer.innerHTML = '';
    results.forEach(result => {
      const item = document.createElement('div');
      item.className = 'search-result';
      item.textContent = result.title;
      resultsContainer.appendChild(item);
    });
  }
}
