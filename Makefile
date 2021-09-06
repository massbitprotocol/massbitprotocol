create-git-hook:
	@echo "Every push to origin need to run the E2E tests"
	@echo "Creating symlink..."
	ln -s -f ../../.githooks/pre-push .git/hooks/pre-push
	@echo "You can remove all the git-hooks with the 'make remove-all-git-hook'" command

remove-all-git-hook:
	@echo "Removing all symlinks..."
	rm .git/hooks/*

test-run-all:
	@echo "Running health check tests ..."
	cd e2e-test/health-check && robot health-check.robot || true
	@echo "Running substrate tests ..."
	cd e2e-test/substrate && robot substrate.robot
	@echo "Running solana tests ..."
	cd e2e-test/solana && robot solana.robot
	@echo "Running ethereum tests ..."
	cd e2e-test/ethereum && robot ethereum.robot

test-run-all-and-up:
	@echo "Run all services"
	bash run.sh
	sleep 10;
	@echo "Running health check tests ..."
	cd e2e-test/health-check && robot health-check.robot || true
	@echo "Running substrate tests ..."
	cd e2e-test/substrate && robot substrate.robot
	@echo "Running solana tests ..."
	cd e2e-test/solana && robot solana.robot
	@echo "Running ethereum tests ..."
	cd e2e-test/ethereum && robot ethereum.robot

test-init:
	@echo "Installing all the dependencies for E2E tests ..."
	pip install robotframework robotframework-requests robotframework-databaselibrary psycopg2 rpaframework robotframework-sshlibrary

create-list-user-example-json-file:
	@echo "Create list user examples json file ..."
	cd user-example && python create_example_json.py

#################### Dev commands ##########################

deploy:
	@echo "Deploy already build indexer $(id)"
	curl --location --request POST 'localhost:5000/deploy/wasm' \
    --header 'Content-Type: application/json' \
    --data-raw '{"configs": {"model": "Factory" }, "compilation_id": "$(id)" }'

run-indexer-manager:
	@echo "Run index-manager"
	cargo run --bin index-manager-main

run-chain-reader:
	@echo "Run chain-reader"
	cargo run --bin chain-reader

run-code-compiler:
	@echo "Run code-compiler"
	cd code-compiler/ && python app.py


services-up:
	@echo "Run all service"
	docker-compose -f docker-compose.min.yml up

services-down:
	@echo "Stop all service"
	docker-compose -f docker-compose.min.yml down
