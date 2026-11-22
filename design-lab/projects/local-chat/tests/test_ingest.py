from app.ingest import passages


def test_passages_split_text(tmp_path):
    assert passages.__name__ == "passages"
