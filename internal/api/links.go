package api

import (
	"encoding/json"
	"net/http"
	"strconv"

	"github.com/go-chi/chi/v5"
)

type createLinkBody struct {
	Type    string         `json:"type"`
	StartID int64          `json:"start_id"`
	EndID   int64          `json:"end_id"`
	Props   map[string]any `json:"props"`
}

func (d *Deps) createLink(w http.ResponseWriter, r *http.Request) {
	var body createLinkBody
	if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
		writeError(w, "400.bad_body", err.Error())
		return
	}
	if body.Props == nil {
		body.Props = map[string]any{}
	}
	// Endpoint validation against schema.
	if relDef, ok := d.Schema.Rels[body.Type]; ok && len(relDef.Endpoints) > 0 {
		sl, el, err := d.Store.LookupLabels(r.Context(), body.StartID, body.EndID)
		if err != nil {
			writeError(w, "404.not_found", "start/end not found")
			return
		}
		if !relDef.EndpointsAllowed(sl, el) {
			writeError(w, "400.endpoint_not_allowed",
				"rel "+body.Type+" not allowed from "+sl+" to "+el)
			return
		}
	}
	id, err := d.Store.CreateLink(r.Context(), body.Type, body.StartID, body.EndID, body.Props)
	if err != nil {
		writeError(w, "502.neo4j", err.Error())
		return
	}
	writeJSON(w, http.StatusOK, map[string]int64{"id": id})
}

func (d *Deps) deleteLink(w http.ResponseWriter, r *http.Request) {
	id, err := strconv.ParseInt(chi.URLParam(r, "id"), 10, 64)
	if err != nil {
		writeError(w, "400.bad_id", err.Error())
		return
	}
	if err := d.Store.DeleteLink(r.Context(), id); err != nil {
		writeError(w, "502.neo4j", err.Error())
		return
	}
	writeJSON(w, http.StatusOK, map[string]bool{"ok": true})
}
