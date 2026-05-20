package api

import (
	"encoding/json"
	"errors"
	"net/http"
	"strconv"

	"github.com/go-chi/chi/v5"

	"github.com/plutonium-guy/kg_editor/internal/schema"
	"github.com/plutonium-guy/kg_editor/internal/store"
)

type createBody struct {
	Label string         `json:"label"`
	Props map[string]any `json:"props"`
}

func (d *Deps) createEntity(w http.ResponseWriter, r *http.Request) {
	var body createBody
	if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
		writeError(w, "400.bad_body", err.Error())
		return
	}
	def, ok := d.Schema.Nodes[body.Label]
	if !ok {
		writeError(w, "400.unknown_label", "unknown node label "+body.Label)
		return
	}
	if errs := schema.ValidateProps(def.Props, body.Props); len(errs) > 0 {
		writeError(w, "400.validation", joinErrs(errs))
		return
	}
	if body.Props == nil {
		body.Props = map[string]any{}
	}
	entity, err := d.Store.CreateNode(r.Context(), body.Label, body.Props)
	if err != nil {
		writeError(w, "502.neo4j", err.Error())
		return
	}
	writeJSON(w, http.StatusOK, map[string]any{
		"id":    entity.ID,
		"label": body.Label,
		"props": entity.Props,
	})
}

type updateBody struct {
	Set   map[string]any `json:"set"`
	Unset []string       `json:"unset"`
}

func (d *Deps) updateEntity(w http.ResponseWriter, r *http.Request) {
	id, err := strconv.ParseInt(chi.URLParam(r, "id"), 10, 64)
	if err != nil {
		writeError(w, "400.bad_id", err.Error())
		return
	}
	var body updateBody
	if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
		writeError(w, "400.bad_body", err.Error())
		return
	}
	// Validate set against schema if we know the label.
	if entity, err := d.Store.GetEntity(r.Context(), id); err == nil && entity != nil && len(entity.Labels) > 0 {
		if def, ok := d.Schema.Nodes[entity.Labels[0]]; ok {
			subset := []schema.FieldSpec{}
			for _, sp := range def.Props {
				if _, has := body.Set[sp.Name]; has {
					subset = append(subset, schema.FieldSpec{
						Name:      sp.Name,
						FieldType: sp.FieldType,
						Required:  false,
					})
				}
			}
			if errs := schema.ValidateProps(subset, body.Set); len(errs) > 0 {
				writeError(w, "400.validation", joinErrs(errs))
				return
			}
		}
	} else if errors.Is(err, store.ErrNotFound) {
		writeError(w, "404.not_found", "no node with id")
		return
	}
	if err := d.Store.UpdateNode(r.Context(), id, body.Set, body.Unset); err != nil {
		writeError(w, "502.neo4j", err.Error())
		return
	}
	writeJSON(w, http.StatusOK, map[string]bool{"ok": true})
}

func (d *Deps) deleteEntity(w http.ResponseWriter, r *http.Request) {
	id, err := strconv.ParseInt(chi.URLParam(r, "id"), 10, 64)
	if err != nil {
		writeError(w, "400.bad_id", err.Error())
		return
	}
	cascade := r.URL.Query().Get("cascade") == "true"
	if err := d.Store.DeleteNode(r.Context(), id, cascade); err != nil {
		writeError(w, "502.neo4j", err.Error())
		return
	}
	writeJSON(w, http.StatusOK, map[string]bool{"ok": true})
}

func (d *Deps) getEntity(w http.ResponseWriter, r *http.Request) {
	id, err := strconv.ParseInt(chi.URLParam(r, "id"), 10, 64)
	if err != nil {
		writeError(w, "400.bad_id", err.Error())
		return
	}
	e, err := d.Store.GetEntity(r.Context(), id)
	if errors.Is(err, store.ErrNotFound) {
		writeError(w, "404.not_found", "no node with id")
		return
	}
	if err != nil {
		writeError(w, "502.neo4j", err.Error())
		return
	}
	writeJSON(w, http.StatusOK, e)
}

func (d *Deps) listEntities(w http.ResponseWriter, r *http.Request) {
	label := r.URL.Query().Get("label")
	q := r.URL.Query().Get("q")
	limit := parseLimit(r.URL.Query().Get("limit"), 50)
	out, err := d.Store.ListEntities(r.Context(), label, q, limit)
	if err != nil {
		writeError(w, "502.neo4j", err.Error())
		return
	}
	if out == nil {
		out = []store.EntityDetail{}
	}
	writeJSON(w, http.StatusOK, out)
}

func parseLimit(s string, def int) int {
	if s == "" {
		return def
	}
	n, err := strconv.Atoi(s)
	if err != nil || n <= 0 {
		return def
	}
	return n
}

func joinErrs(errs []string) string {
	out := ""
	for i, e := range errs {
		if i > 0 {
			out += "; "
		}
		out += e
	}
	return out
}
