package api

import (
	"net/http"
)

func (d *Deps) health(w http.ResponseWriter, r *http.Request) {
	status := "reachable"
	if err := d.Store.Ping(r.Context()); err != nil {
		status = "down"
	}
	writeJSON(w, http.StatusOK, map[string]string{"status": "ok", "neo4j": status})
}
