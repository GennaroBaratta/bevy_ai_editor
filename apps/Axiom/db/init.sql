CREATE TABLE timeseries_data (
    id SERIAL PRIMARY KEY,
    ts TIMESTAMP NOT NULL,
    value DOUBLE PRECISION NOT NULL
);

CREATE TABLE alerts (
    id SERIAL PRIMARY KEY,
    msg TEXT NOT NULL,
    level VARCHAR(50) NOT NULL
);

INSERT INTO alerts (msg, level) VALUES
    ('System Overload', 'CRITICAL'),
    ('System Overload', 'CRITICAL'),
    ('System Overload', 'CRITICAL'),
    ('System Overload', 'CRITICAL'),
    ('System Overload', 'CRITICAL');
