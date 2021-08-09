import psycopg2

def delete_table_if_exists(table_name):
    conn = None
    try:
        print('Connecting to the PostgreSQL database...')
        conn = psycopg2.connect(
            host="localhost",
            database="graph-node",
            user="graph-node",
            password="let-me-in")
        cur = conn.cursor()
        print('Deleting table if exists:')
        cur.execute('DROP TABLE IF EXISTS ' + table_name + ' CASCADE')
        conn.commit()
        cur.close()
    except (Exception, psycopg2.DatabaseError) as error:
        print(error)
    finally:
        if conn is not None:
            conn.close()
            print('Database connection closed.')
