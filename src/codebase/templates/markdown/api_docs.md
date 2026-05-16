<!-- @LITE_DESC: API documentation template with endpoints, parameters, responses, and authentication -->
<!-- @LITE_SCENE: Documenting REST APIs, microservice interfaces, backend endpoints -->
<!-- @LITE_TAGS: markdown, api, documentation, rest, endpoints, backend -->

# API Documentation

Base URL: `http://localhost:8080/api/v1`

## Authentication

All authenticated endpoints require a Bearer token:

```
Authorization: Bearer <your-api-key>
```

## Endpoints

### List Resources

`GET /resources`

**Query Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `page` | integer | No | Page number (default: 1) |
| `limit` | integer | No | Items per page (default: 20, max: 100) |
| `sort` | string | No | Sort field (e.g., `created_at`, `name`) |
| `order` | string | No | `asc` or `desc` (default: `desc`) |

**Response (200):**
```json
{
  "data": [
    {
      "id": "uuid-string",
      "name": "Resource Name",
      "created_at": "2024-01-15T10:30:00Z",
      "updated_at": "2024-01-15T10:30:00Z"
    }
  ],
  "pagination": {
    "page": 1,
    "limit": 20,
    "total": 100,
    "pages": 5
  }
}
```

### Create Resource

`POST /resources`

**Request Body:**
```json
{
  "name": "New Resource",
  "description": "Optional description",
  "tags": ["tag1", "tag2"]
}
```

**Response (201):**
```json
{
  "id": "new-uuid",
  "name": "New Resource",
  "created_at": "2024-01-15T10:30:00Z"
}
```

**Error Response (400):**
```json
{
  "error": "Validation failed",
  "details": [
    {"field": "name", "message": "Name is required"}
  ]
}
```

### Get Resource

`GET /resources/{id}`

**Response (200):**
```json
{
  "id": "uuid-string",
  "name": "Resource Name",
  "description": "Full description",
  "tags": ["tag1"],
  "created_at": "2024-01-15T10:30:00Z"
}
```

### Update Resource

`PUT /resources/{id}`

**Request Body:**
```json
{
  "name": "Updated Name",
  "description": "Updated description"
}
```

### Delete Resource

`DELETE /resources/{id}`

**Response (204):** No content

## Error Codes

| Code | Meaning |
|------|---------|
| 400 | Bad Request - Invalid parameters |
| 401 | Unauthorized - Missing or invalid token |
| 403 | Forbidden - Insufficient permissions |
| 404 | Not Found - Resource does not exist |
| 409 | Conflict - Duplicate resource |
| 422 | Unprocessable Entity - Validation error |
| 500 | Internal Server Error |

## Rate Limiting

- 100 requests per minute for authenticated users
- 20 requests per minute for unauthenticated users
- Rate limit headers: `X-RateLimit-Limit`, `X-RateLimit-Remaining`, `X-RateLimit-Reset`
