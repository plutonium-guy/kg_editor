package api

import (
	"encoding/json"
	"net/http"
)

type queryBody struct {
	Cypher string         `json:"cypher"`
	Params map[string]any `json:"params"`
}

func (d *Deps) query(w http.ResponseWriter, r *http.Request) {
	var body queryBody
	if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
		writeError(w, "400.bad_body", err.Error())
		return
	}
	if body.Cypher == "" {
		writeError(w, "400.empty_cypher", "cypher required")
		return
	}
	if body.Params == nil {
		body.Params = map[string]any{}
	}
	rows, err := d.Store.Query(r.Context(), body.Cypher, body.Params)
	if err != nil {
		writeError(w, "422.cypher_error", err.Error())
		return
	}
	if rows == nil {
		rows = []map[string]any{}
	}
	writeJSON(w, http.StatusOK, map[string]any{"rows": rows})
}
