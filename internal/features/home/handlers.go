package home

import "net/http"

type Handler struct{}

func NewHandler() *Handler {
	return &Handler{}
}

func (h *Handler) Index(w http.ResponseWriter, r *http.Request) {
	w.Write([]byte("Hello world"))
}
