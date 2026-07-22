CREATE EXTENSION IF NOT EXISTS pgcrypto
CREATE EXTENSION IF NOT EXISTS citext

CREATE OR REPLACE FUNCTION prefixed_nanoid(
    prefix text DEFAULT 'id', -- Prefix to use before nanoid e.g. usr_xxxxxx
    size int DEFAULT 16, -- The desired length of the generated string.
    -- The set of characters to choose from for generating the string.
    alphabet text DEFAULT '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz'
) RETURNS text -- A randomly generated NanoId String
LANGUAGE plpgsql VOLATILE PARALLEL SAFE AS $$
DECLARE
    idBuilder      text := '';
    counter        int  := 0;
    bytes          bytea;
    alphabetIndex  int;
    alphabetArray  text[];
    alphabetLength int  := 64;
    mask           int  := 63; -- The mask used for mapping random bytes to alphabet indices. Should be (2^n) - 1 where n is a power of 2 less than or equal to the alphabet size.
    step           int  := 34; -- The number of random bytes to generate in each iteration. A larger value may speed up the function but increase memory usage.
BEGIN
    alphabetArray := regexp_split_to_array(alphabet, '');
    alphabetLength := array_length(alphabetArray, 1);

    LOOP
        bytes := gen_random_bytes(step);
        FOR counter IN 0..step - 1
            LOOP
                alphabetIndex := (get_byte(bytes, counter) & mask) + 1;
                IF alphabetIndex <= alphabetLength THEN
                    idBuilder := idBuilder || alphabetArray[alphabetIndex];
                    IF length(idBuilder) = size THEN
                        RETURN prefix || '_' || idBuilder;
                    END IF;
                END IF;
            END LOOP;
    END LOOP;
END
$$;

CREATE OR REPLACE FUNCTION is_prefixed_pid(
    value text, prefix text, size int DEFAULT 16
) RETURNS boolean LANGUAGE sql IMMUTABLE STRICT AS $$
    SELECT value ~ ('^' || prefix || '_[1-9A-HJ-NP-Za-km-z]{' || size || '}$')
    $$;

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
