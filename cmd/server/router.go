package main

import (
	"context"
	"errors"
	"log/slog"

	homeFeature "pulsar/internal/features/home"

	"github.com/go-chi/chi/v5"
)

func SetupRoutes(ctx context.Context, router chi.Router) {
	if err := errors.Join(
		homeFeature.SetupRoutes(ctx, router),
	); err != nil {
		slog.Error("error setting up routes", err)
		panic(1)
	}
}
