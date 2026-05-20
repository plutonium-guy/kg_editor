.PHONY: neo4j-up neo4j-down test-core test-native test-wasm fmt clippy doc

neo4j-up:
	docker run -d --name kg-neo4j -p 7687:7687 -p 7474:7474 \
		-e NEO4J_AUTH=neo4j/test neo4j:5-community

neo4j-down:
	docker rm -f kg-neo4j

test-core:
	cargo test -p kg-core

test-native:
	cargo test -p kg-neo4j --features native

test-wasm:
	wasm-pack test --headless --chrome crates/kg-neo4j --features wasm

fmt:
	cargo fmt --all

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

doc:
	cargo doc --no-deps --all-features
