## Development
### Run all services
Run all service with a single command. For testing purpose.
```bash
sh run.sh
```

### Deploy
Deploy the indexer with id, in case the indexer's files already successfully build once. This is for reducing rebuild time.

```bash
make dev-deploy id=54e42a73317d80d1cf8289b49af96302
```

### Manual deploy
```bash
cd scripts
./manual-deploy.sh
```