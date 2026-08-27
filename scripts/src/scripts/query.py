from vespa.io import VespaQueryResponse

from scripts import app, eprint


def query():
    vespa = app.load()

    with vespa.syncio() as session:
        eprint("> Launching query")
        response: VespaQueryResponse = session.query(
            body={
                "yql": "select title, body from doc where userQuery()",
                "query": "Is statin use connected to breast cancer?",
                "ranking": "bm25",
                "presentation.timing": True,
            },
        )

        if response.is_successful():
            import json

            print(json.dumps(response.json))
        else:
            print("Query has failed")


if __name__ == "__main__":
    query()
