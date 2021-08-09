test-all:
	@echo "Installing Robotframework prerequisites..."
	pip install robotframework robotframework-requests robotframework-databaselibrary
	@echo "Running substrate tests ..."
	cd substrate && robot substrate.robot || true
	@echo "Running solana tests ..."
	cd solana && robot solana.robot || true
	@echo "Running health check tests ..."
	cd health-check && robot health-check.robot || true
