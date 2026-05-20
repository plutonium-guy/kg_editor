package api

import "net/http"

func (d *Deps) getSchema(w http.ResponseWriter, _ *http.Request) {
	writeJSON(w, http.StatusOK, d.Schema)
}
