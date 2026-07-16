CREATE FUNCTION update_updated_at_column() RETURNS trigger AS $$
BEGIN NEW .updated_at = now();
RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TABLE users (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    full_name text NOT NULL,
    email text NOT NULL UNIQUE,
    created_at timestamptz DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TRIGGER update_users_updated_at BEFORE
UPDATE ON users FOR EACH ROW
EXECUTE FUNCTION update_updated_at_column();
