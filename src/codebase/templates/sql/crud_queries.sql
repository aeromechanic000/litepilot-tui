-- @LITE_DESC SQL CRUD queries with joins, filters, transactions, pagination, and aggregates
-- @LITE_SCENE Common database operations for application development
-- @LITE_TAGS sql, queries, crud, select, insert

-- ============================================================================
-- INSERT OPERATIONS
-- ============================================================================

-- Simple insert
INSERT INTO users (username, email, password_hash, full_name)
VALUES ('john_doe', 'john@example.com', 'hashed_password', 'John Doe');

-- Insert with returning clause
INSERT INTO posts (user_id, title, content, status, published_at)
VALUES (
    'user-uuid',
    'My First Post',
    'This is the content of my first post.',
    'published',
    CURRENT_TIMESTAMP
)
RETURNING id, created_at;

-- Bulk insert
INSERT INTO tags (name, slug) VALUES
    ('JavaScript', 'javascript'),
    ('Python', 'python'),
    ('Databases', 'databases');

-- Insert with on conflict upsert
INSERT INTO users (username, email, password_hash)
VALUES ('jane_doe', 'jane@example.com', 'hashed_password')
ON CONFLICT (username)
DO UPDATE SET
    email = EXCLUDED.email,
    updated_at = CURRENT_TIMESTAMP;

-- ============================================================================
-- SELECT OPERATIONS
-- ============================================================================

-- Simple select with where clause
SELECT id, username, email, full_name
FROM users
WHERE is_active = true
    AND created_at >= CURRENT_DATE - INTERVAL '30 days'
ORDER BY created_at DESC;

-- Select with joins
SELECT
    p.id,
    p.title,
    p.slug,
    p.content,
    u.username as author_username,
    u.full_name as author_name,
    COUNT(c.id) as comment_count
FROM posts p
JOIN users u ON p.user_id = u.id
LEFT JOIN comments c ON p.id = c.post_id AND c.status = 'approved'
WHERE p.status = 'published'
    AND p.published_at <= CURRENT_TIMESTAMP
GROUP BY p.id, u.id
ORDER BY p.published_at DESC;

-- Select with multiple joins
SELECT
    p.id,
    p.title,
    p.slug,
    p.content,
    u.username as author_username,
    u.full_name as author_name,
    ARRAY_AGG(DISTINCT t.name) as tags,
    COUNT(DISTINCT c.id) as comment_count,
    COUNT(DISTINCT pt.user_id) FILTER (WHERE pt.type = 'like') as like_count
FROM posts p
JOIN users u ON p.user_id = u.id
LEFT JOIN post_tags pt_join ON p.id = pt_join.post_id
LEFT JOIN tags t ON pt_join.tag_id = t.id
LEFT JOIN comments c ON p.id = c.post_id AND c.status = 'approved'
LEFT JOIN post_tags pt ON p.id = pt.post_id
WHERE p.status = 'published'
GROUP BY p.id, u.id
ORDER BY p.published_at DESC;

-- Select with subquery
SELECT
    u.username,
    u.full_name,
    u.email,
    p.post_count,
    p.last_post_date
FROM users u
LEFT JOIN (
    SELECT
        user_id,
        COUNT(*) as post_count,
        MAX(created_at) as last_post_date
    FROM posts
    WHERE status = 'published'
    GROUP BY user_id
) p ON u.id = p.user_id
WHERE u.is_active = true
ORDER BY post_count DESC;

-- ============================================================================
-- FILTERING AND SEARCHING
-- ============================================================================

-- Text search with LIKE
SELECT title, slug, content
FROM posts
WHERE title ILIKE '%tutorial%'
    OR content ILIKE '%tutorial%'
    AND status = 'published';

-- Full-text search
SELECT
    title,
    slug,
    content,
    ts_rank(textsearchable_index_col, query) as rank
FROM posts,
     to_tsquery('english', 'database | tutorial') query
WHERE textsearchable_index_col @@ query
    AND status = 'published'
ORDER BY rank DESC
LIMIT 20;

-- Filter by date range
SELECT title, published_at
FROM posts
WHERE published_at >= '2024-01-01'
    AND published_at < '2024-02-01'
    AND status = 'published'
ORDER BY published_at DESC;

-- Complex WHERE conditions
SELECT id, title, status, published_at
FROM posts
WHERE (status = 'published' AND published_at <= CURRENT_TIMESTAMP)
    OR (status = 'scheduled' AND published_at > CURRENT_TIMESTAMP)
    OR (status = 'draft' AND user_id = 'current-user-id')
ORDER BY published_at DESC;

-- IN clause with subquery
SELECT username, full_name
FROM users
WHERE id IN (
    SELECT DISTINCT user_id
    FROM posts
    WHERE status = 'published'
        AND created_at >= CURRENT_DATE - INTERVAL '7 days'
);

-- ============================================================================
-- UPDATE OPERATIONS
-- ============================================================================

-- Simple update
UPDATE posts
SET status = 'published',
    published_at = CURRENT_TIMESTAMP
WHERE id = 'post-uuid';

-- Update with join
UPDATE posts
SET view_count = view_count + 1
WHERE id = 'post-uuid';

-- Update multiple columns
UPDATE users
SET
    full_name = 'Updated Name',
    bio = 'New bio text',
    updated_at = CURRENT_TIMESTAMP
WHERE id = 'user-uuid';

-- Conditional update
UPDATE posts
SET status = CASE
    WHEN published_at <= CURRENT_TIMESTAMP THEN 'published'
    WHEN published_at > CURRENT_TIMESTAMP THEN 'scheduled'
    ELSE 'draft'
END
WHERE status IN ('draft', 'scheduled');

-- Update with returning clause
UPDATE comments
SET status = 'approved',
    updated_at = CURRENT_TIMESTAMP
WHERE id = 'comment-uuid'
RETURNING id, status, updated_at;

-- Bulk update
UPDATE posts
SET status = 'archived'
WHERE published_at < CURRENT_TIMESTAMP - INTERVAL '1 year'
    AND status = 'published';

-- ============================================================================
-- DELETE OPERATIONS
-- ============================================================================

-- Simple delete
DELETE FROM comments
WHERE created_at < CURRENT_TIMESTAMP - INTERVAL '1 year'
    AND status = 'pending';

-- Delete with returning
DELETE FROM post_tags
WHERE post_id = 'post-uuid'
RETURNING tag_id;

-- Conditional delete with limit
DELETE FROM comments
WHERE id IN (
    SELECT id
    FROM comments
    WHERE status = 'spam'
        AND created_at < CURRENT_TIMESTAMP - INTERVAL '30 days'
    LIMIT 1000
);

-- ============================================================================
-- AGGREGATION AND GROUPING
-- ============================================================================

-- Count with grouping
SELECT
    DATE_TRUNC('month', created_at) as month,
    COUNT(*) as post_count
FROM posts
WHERE status = 'published'
    AND created_at >= CURRENT_DATE - INTERVAL '12 months'
GROUP BY DATE_TRUNC('month', created_at)
ORDER BY month DESC;

-- Multiple aggregations
SELECT
    u.username,
    COUNT(p.id) as total_posts,
    COUNT(p.id) FILTER (WHERE p.status = 'published') as published_posts,
    COALESCE(SUM(p.view_count), 0) as total_views,
    COALESCE(AVG(p.view_count), 0) as avg_views
FROM users u
LEFT JOIN posts p ON u.id = p.user_id
GROUP BY u.id, u.username
HAVING COUNT(p.id) > 0
ORDER BY total_views DESC;

-- Percentiles and statistics
SELECT
    COUNT(*) as total_posts,
    AVG(view_count) as avg_views,
    PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY view_count) as median_views,
    PERCENTILE_CONT(0.90) WITHIN GROUP (ORDER BY view_count) as p90_views,
    MAX(view_count) as max_views,
    MIN(view_count) as min_views
FROM posts
WHERE status = 'published';

-- ============================================================================
-- PAGINATION
-- ============================================================================

-- Offset-based pagination
SELECT
    p.id,
    p.title,
    p.slug,
    u.username as author
FROM posts p
JOIN users u ON p.user_id = u.id
WHERE p.status = 'published'
ORDER BY p.published_at DESC
LIMIT 20 OFFSET 0;

-- Cursor-based pagination (more efficient)
SELECT
    p.id,
    p.title,
    p.slug,
    u.username as author
FROM posts p
JOIN users u ON p.user_id = u.id
WHERE p.status = 'published'
    AND p.published_at < 'last-post-date'
ORDER BY p.published_at DESC
LIMIT 20;

-- Window function for row numbers
SELECT *
FROM (
    SELECT
        p.id,
        p.title,
        p.slug,
        ROW_NUMBER() OVER (ORDER BY p.published_at DESC) as row_num
    FROM posts p
    WHERE p.status = 'published'
) numbered
WHERE row_num BETWEEN 21 AND 40;

-- ============================================================================
-- TRANSACTIONS
-- ============================================================================

-- Transaction with error handling
BEGIN;

-- Insert new post
INSERT INTO posts (user_id, title, content, status, published_at)
VALUES ('user-uuid', 'New Post', 'Content here', 'published', CURRENT_TIMESTAMP);

-- Get the new post ID
DO $$
DECLARE
    new_post_id UUID;
BEGIN
    SELECT LAST_INSERT_ID() INTO new_post_id;

    -- Add tags
    INSERT INTO post_tags (post_id, tag_id)
    SELECT new_post_id, id FROM tags WHERE name IN ('JavaScript', 'Tutorial');

    -- Log activity
    INSERT INTO activity_log (user_id, action, entity_type, entity_id)
    VALUES ('user-uuid', 'created', 'post', new_post_id);
END $$;

COMMIT;

-- Transaction with rollback on error
BEGIN;
    -- Savepoint for partial rollback
    SAVEPOINT post_insert;

    BEGIN
        -- Attempt to insert post
        INSERT INTO posts (user_id, title, content, status)
        VALUES ('user-uuid', 'Title', 'Content', 'published');
    EXCEPTION WHEN OTHERS THEN
        ROLLBACK TO SAVEPOINT post_insert;
        RAISE NOTICE 'Post insert failed, rolling back';
    END;

    -- Continue with other operations
    UPDATE users SET last_post_at = CURRENT_TIMESTAMP WHERE id = 'user-uuid';

COMMIT;

-- ============================================================================
-- COMMON PATTERNS
-- ============================================================================

-- Upsert pattern
INSERT INTO user_preferences (user_id, theme, language)
VALUES ('user-uuid', 'dark', 'en')
ON CONFLICT (user_id)
DO UPDATE SET
    theme = EXCLUDED.theme,
    language = EXCLUDED.language,
    updated_at = CURRENT_TIMESTAMP;

-- Soft delete
UPDATE comments
SET status = 'deleted',
    content = '[deleted]',
    updated_at = CURRENT_TIMESTAMP
WHERE id = 'comment-uuid';

-- Hierarchical data (self-referencing)
WITH RECURSIVE comment_tree AS (
    -- Base case: top-level comments
    SELECT
        id,
        post_id,
        user_id,
        content,
        parent_id,
        0 as level,
        ARRAY[id] as path
    FROM comments
    WHERE post_id = 'post-uuid'
        AND parent_id IS NULL
        AND status = 'approved'

    UNION ALL

    -- Recursive case: child comments
    SELECT
        c.id,
        c.post_id,
        c.user_id,
        c.content,
        c.parent_id,
        ct.level + 1,
        ct.path || c.id
    FROM comments c
    JOIN comment_tree ct ON c.parent_id = ct.id
    WHERE c.status = 'approved'
)
SELECT * FROM comment_tree ORDER BY path;

-- Materialized path pattern (alternative to hierarchical)
SELECT
    id,
    content,
    user_id,
    -- Count the number of / separators to determine depth
    LENGTH(path) - LENGTH(REPLACE(path, '/', '')) as depth
FROM comments
WHERE path LIKE 'post-id/comment-id/%'
ORDER BY path;