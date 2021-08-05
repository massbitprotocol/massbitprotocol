e2e-test-integrate-git-hook:
	@echo "Setting up local git push with a git hook constraint"
	@echo "Every push to origin need to run the E2E robot framework test"
	@echo "Creating symlink..."
	ln -s -f ../../.githooks/pre-push .git/hooks/pre-push