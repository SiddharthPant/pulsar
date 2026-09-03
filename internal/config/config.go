package config

import (
	"log/slog"
	"os"
	"strconv"
	"strings"
	"sync"
)

type App struct {
	Host     string
	Port     int
	LogLevel slog.Level
}

type Config struct {
	App
}

var (
	Env  *Config
	once sync.Once
)

func init() {
	once.Do(func() {
		Env = loadConfig()
	})
}

func getEnvString(key string) string {
	value, ok := os.LookupEnv(key)
	if ok == true {
		if strings.TrimSpace(value) != "" {
			return value
		} else {
			slog.Error("env var is set but empty", "envVar", key, "value", value)
		}
	} else {
		slog.Error("env var is not set", "envVar", key, "value", value)
	}
	panic(1)
}

func getEnvData[T any](key string, parser func(value string) (T, error)) T {
	value := getEnvString(key)
	parsedValue, err := parser(value)
	if err == nil {
		return parsedValue
	}

	slog.Error("Parsing failed", "envVar", key, "value", value, "error", err.Error())
	panic(1)
}

func loadConfig() *Config {
	config := &Config{
		App: App{
			Host: getEnvString("APP_HOST"),
			Port: getEnvData("APP_PORT", strconv.Atoi),
			LogLevel: getEnvData("APP_LOG_LEVEL", func(rawValue string) (slog.Level, error) {
				var level slog.Level
				err := level.UnmarshalText([]byte(rawValue))
				return level, err
			}),
		},
	}

	return config
}
