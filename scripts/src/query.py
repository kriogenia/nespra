from vespa.io import VespaQueryResponse

import app


def query():
    vespa = app.load()

    with vespa.syncio() as session:
        print("> Launching query")
        response: VespaQueryResponse = session.query(
            body={
                "yql": "select title, body from doc where userQuery()",
                "query": "Is statin use connected to breast cancer?",
                "ranking": "bm25",
                "presentation.timing": True,
            },
        )

        if response.is_successful():
            print(response.hits)
        else:
            print("Query has failed")
