```genericsql
DROP TABLE IF EXISTS users CASCADE;

CREATE TABLE IF NOT EXISTS users
(
    user_id    BIGINT PRIMARY KEY       NOT NULL,

    username   VARCHAR(64)              NOT NULL,
    first_name VARCHAR(255)             NOT NULL,
    last_name  VARCHAR(255),
    subscribes integer NULL,

    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_users_username ON users (username);
```
