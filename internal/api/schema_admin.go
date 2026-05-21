package api

import (
	"encoding/json"
	"net/http"

	"github.com/go-chi/chi/v5"

	"github.com/plutonium-guy/kg_editor/internal/schema"
)

func (d *Deps) getSchema(w http.ResponseWriter, _ *http.Request) {
	writeJSON(w, http.StatusOK, d.Schema())
}

func (d *Deps) putSchema(w http.ResponseWriter, r *http.Request) {
	var f schema.File
	if err := json.NewDecoder(r.Body).Decode(&f); err != nil {
		writeError(w, "400.bad_body", err.Error())
		return
	}
	if f.Nodes == nil {
		f.Nodes = map[string]schema.NodeDef{}
	}
	if f.Rels == nil {
		f.Rels = map[string]schema.RelDef{}
	}
	if err := f.Validate(); err != nil {
		writeError(w, "400.validation", err.Error())
		return
	}
	if err := d.SchemaStore.Save(r.Context(), &f); err != nil {
		writeError(w, "502.neo4j", err.Error())
		return
	}
	loaded, err := d.SchemaStore.Load(r.Context())
	if err != nil {
		writeError(w, "502.neo4j", err.Error())
		return
	}
	d.SetSchema(loaded)
	writeJSON(w, http.StatusOK, loaded)
}

func (d *Deps) reloadSchema(w http.ResponseWriter, r *http.Request) {
	loaded, err := d.SchemaStore.Load(r.Context())
	if err != nil {
		writeError(w, "502.neo4j", err.Error())
		return
	}
	d.SetSchema(loaded)
	writeJSON(w, http.StatusOK, loaded)
}

type nodeBody struct {
	Name string         `json:"name"`
	Def  schema.NodeDef `json:"def"`
}

func (d *Deps) createSchemaNode(w http.ResponseWriter, r *http.Request) {
	var b nodeBody
	if err := json.NewDecoder(r.Body).Decode(&b); err != nil {
		writeError(w, "400.bad_body", err.Error())
		return
	}
	if b.Name == "" {
		writeError(w, "400.missing_name", "name required")
		return
	}
	current := d.Schema()
	if _, exists := current.Nodes[b.Name]; exists {
		writeError(w, "400.exists", "label already exists; use PUT")
		return
	}
	if err := d.SchemaStore.SaveNode(r.Context(), b.Name, b.Def); err != nil {
		writeError(w, "502.neo4j", err.Error())
		return
	}
	d.refresh(r)
	writeJSON(w, http.StatusOK, d.Schema())
}

func (d *Deps) updateSchemaNode(w http.ResponseWriter, r *http.Request) {
	label := chi.URLParam(r, "label")
	var def schema.NodeDef
	if err := json.NewDecoder(r.Body).Decode(&def); err != nil {
		writeError(w, "400.bad_body", err.Error())
		return
	}
	if err := d.SchemaStore.SaveNode(r.Context(), label, def); err != nil {
		writeError(w, "502.neo4j", err.Error())
		return
	}
	d.refresh(r)
	writeJSON(w, http.StatusOK, d.Schema())
}

func (d *Deps) deleteSchemaNode(w http.ResponseWriter, r *http.Request) {
	label := chi.URLParam(r, "label")
	if err := d.SchemaStore.DeleteNode(r.Context(), label); err != nil {
		writeError(w, "502.neo4j", err.Error())
		return
	}
	d.refresh(r)
	writeJSON(w, http.StatusOK, d.Schema())
}

type relBody struct {
	Type string        `json:"type"`
	Def  schema.RelDef `json:"def"`
}

func (d *Deps) createSchemaRel(w http.ResponseWriter, r *http.Request) {
	var b relBody
	if err := json.NewDecoder(r.Body).Decode(&b); err != nil {
		writeError(w, "400.bad_body", err.Error())
		return
	}
	if b.Type == "" {
		writeError(w, "400.missing_type", "type required")
		return
	}
	current := d.Schema()
	if _, exists := current.Rels[b.Type]; exists {
		writeError(w, "400.exists", "type already exists; use PUT")
		return
	}
	// Verify endpoints reference known labels.
	for _, ep := range b.Def.Endpoints {
		if _, ok := current.Nodes[ep[0]]; !ok {
			writeError(w, "400.bad_endpoint", "unknown label "+ep[0])
			return
		}
		if _, ok := current.Nodes[ep[1]]; !ok {
			writeError(w, "400.bad_endpoint", "unknown label "+ep[1])
			return
		}
	}
	if err := d.SchemaStore.SaveRel(r.Context(), b.Type, b.Def); err != nil {
		writeError(w, "502.neo4j", err.Error())
		return
	}
	d.refresh(r)
	writeJSON(w, http.StatusOK, d.Schema())
}

func (d *Deps) updateSchemaRel(w http.ResponseWriter, r *http.Request) {
	ty := chi.URLParam(r, "type")
	var def schema.RelDef
	if err := json.NewDecoder(r.Body).Decode(&def); err != nil {
		writeError(w, "400.bad_body", err.Error())
		return
	}
	current := d.Schema()
	for _, ep := range def.Endpoints {
		if _, ok := current.Nodes[ep[0]]; !ok {
			writeError(w, "400.bad_endpoint", "unknown label "+ep[0])
			return
		}
		if _, ok := current.Nodes[ep[1]]; !ok {
			writeError(w, "400.bad_endpoint", "unknown label "+ep[1])
			return
		}
	}
	if err := d.SchemaStore.SaveRel(r.Context(), ty, def); err != nil {
		writeError(w, "502.neo4j", err.Error())
		return
	}
	d.refresh(r)
	writeJSON(w, http.StatusOK, d.Schema())
}

func (d *Deps) deleteSchemaRel(w http.ResponseWriter, r *http.Request) {
	ty := chi.URLParam(r, "type")
	if err := d.SchemaStore.DeleteRel(r.Context(), ty); err != nil {
		writeError(w, "502.neo4j", err.Error())
		return
	}
	d.refresh(r)
	writeJSON(w, http.StatusOK, d.Schema())
}

// refresh reloads the schema from Neo4j and swaps the atomic pointer.
func (d *Deps) refresh(r *http.Request) {
	if loaded, err := d.SchemaStore.Load(r.Context()); err == nil {
		d.SetSchema(loaded)
	}
}
