## Developers
- To add extension on init for Postgres, you can add them in pg_add_extension.sql
- This script only run when Postgres is created the first time, so you can either do 1 of the following:
    - `exec /bin/sh` into the running docker and run the query script yourself
    - or delete the docker container, image, and volume of postgres. Then start docker-compose again (it will automatically be added)