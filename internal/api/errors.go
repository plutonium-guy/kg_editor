package api

import (
	"encoding/json"
	"net/http"
	"strings"
)

type apiError struct {
	Code    string `json:"code"`
	Message string `json:"message"`
}

func writeError(w http.ResponseWriter, code, message string) {
	status := http.StatusInternalServerError
	switch {
	case strings.HasPrefix(code, "400."):
		status = http.StatusBadRequest
	case strings.HasPrefix(code, "404."):
		status = http.StatusNotFound
	case strings.HasPrefix(code, "422."):
		status = http.StatusUnprocessableEntity
	case strings.HasPrefix(code, "502."):
		status = http.StatusBadGateway
	}
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(map[string]apiError{"error": {Code: code, Message: message}})
}

func writeJSON(w http.ResponseWriter, status int, body any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(body)
}
