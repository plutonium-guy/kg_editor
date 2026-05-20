.PHONY: neo4j-up neo4j-down go-build go-test go-test-int kg-server-run kg-mcp-run webui-dev webui-build fmt lint

neo4j-up:
	docker run -d --name kg-neo4j -p 7687:7687 -p 7474:7474 \
		-e NEO4J_AUTH=neo4j/testtest neo4j:5-community

neo4j-down:
	docker rm -f kg-neo4j

go-build:
	go build ./...

go-test:
	go test ./internal/schema/... ./internal/api/...

go-test-int:
	go test -tags=integration ./...

kg-server-run:
	KG_SCHEMA=$(PWD)/kg-schema.yaml NEO4J_PASSWORD=testtest go run ./cmd/kg-server

kg-mcp-run:
	KG_SERVER_URL=http://localhost:9000 go run ./cmd/kg-mcp

webui-dev:
	cd webui && npm run dev

webui-build:
	cd webui && npm run build

fmt:
	gofmt -w .

lint:
	go vet ./...
