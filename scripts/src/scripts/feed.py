from datasets import load_dataset
from vespa.io import VespaResponse

from scripts import app, eprint


def feed():
    eprint("> Loading dataset")
    dataset = load_dataset("BeIR/nfcorpus", "corpus", split="corpus", streaming=True)
    vespa_feed = dataset.map(
        lambda x: {
            "id": x["_id"],
            "fields": {"title": x["title"], "body": x["text"], "id": x["_id"]},
        }
    ).take(100)

    vespa = app.load()

    def callback(response: VespaResponse, id: str):
        if not response.is_successful():
            eprint(f"Error when feeding document {id}: {response.get_json()}")

    eprint("> Feeding dataset")
    vespa.feed_iterable(
        vespa_feed,
        schema="doc",
        namespace="tutorial",
        callback=callback,
    )
    eprint("> Feeding completed")


if __name__ == "__main__":
    feed()
