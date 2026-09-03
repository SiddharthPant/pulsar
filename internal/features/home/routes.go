package home

import (
	"context"

	"github.com/go-chi/chi/v5"
)

func SetupRoutes(ctx context.Context, router chi.Router) error {
	handler := NewHandler()

	router.Get("/", handler.Index)

	return nil
}
