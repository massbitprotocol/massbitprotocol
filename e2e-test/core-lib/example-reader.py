import urllib.parse
import os

def read_index_example(path):
    # Read mapping.rs
    mapping_file = open(os.path.join(path, "mapping.rs"))
    mapping = urllib.parse.quote_plus(mapping_file.read())
    mapping_file.close()

    # Read project.yaml
    project_file = open(os.path.join(path, "project.yaml"))
    project = urllib.parse.quote_plus(project_file.read())
    project_file.close()

    # Read schema.graphql
    schema_file = open(os.path.join(path, "schema.graphql"))
    schema = urllib.parse.quote_plus(schema_file.read())
    schema_file.close()

    payload = {
        "mapping.rs": mapping,
        "project.yaml": project,
        "schema.graphql": schema
    }
    print(payload)
    return payload