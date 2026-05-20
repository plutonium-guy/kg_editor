package api

import "net/http"

func (d *Deps) search(w http.ResponseWriter, r *http.Request) {
	q := r.URL.Query().Get("q")
	limit := parseLimit(r.URL.Query().Get("limit"), 20)
	out, err := d.Store.ListEntities(r.Context(), "", q, limit)
	if err != nil {
		writeError(w, "502.neo4j", err.Error())
		return
	}
	writeJSON(w, http.StatusOK, out)
}
