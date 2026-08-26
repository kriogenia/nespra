from vespa.io import VespaResponse
from datasets import load_dataset

import app


def feed():
    print("> Loading dataset")
    dataset = load_dataset("BeIR/nfcorpus", "corpus", split="corpus", streaming=True)
    vespa_feed = dataset.map(
        lambda x: {
            "id": x["_id"],
            "fields": {"title": x["title"], "body": x["text"], "id": x["_id"]},
        }
    ).take(100)

    vespa = app.load()

    def callback(response: VespaResponse, id: str):
        if response.is_successful():
            print("Feeding completed")
        else:
            print(f"Error when feeding document {id}: {response.get_json()}")

    print("> Feeding dataset")
    vespa.feed_iterable(
        vespa_feed,
        schema="doc",
        namespace="tutorial",
        callback=callback,
    )
