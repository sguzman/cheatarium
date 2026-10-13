#!/usr/bin/env python3
"""Synthetic ZIP/XML structure tests; no upstream source access."""
import io
import unittest
import zipfile

from audit_mame_cheat_xml import audit_zip, local_name


def zip_bytes(items):
    stream = io.BytesIO()
    with zipfile.ZipFile(stream, "w") as archive:
        for path, raw in items:
            archive.writestr(path, raw)
    return stream.getvalue()


class TestMameXMLAudit(unittest.TestCase):
    def test_counts_exact_subtrees_and_unchanged_zip_byte_repeats(self):
        x = (b'<mamecheat version="1"><cheat desc="A"><script>'
             b'<action>do</action></script></cheat><cheat desc="B"/>'
             b'</mamecheat>')
        y = b'<mamecheat version="1"><cheat desc="A"><script><action>do</action></script></cheat></mamecheat>'
        result = audit_zip(zip_bytes([
            ("a.xml", x), ("a-duplicate.xml", x), ("b.xml", y),
        ]))
        self.assertEqual(result["xml_member_occurrences"], 3)
        self.assertEqual(result["distinct_exact_member_file_bytes"], 2)
        self.assertEqual(result["cheat_element_occurrences"], 5)
        self.assertEqual(result["distinct_exact_serialized_cheat_xml_subtrees"], 2)
        self.assertEqual(result["action_element_occurrences"], 3)
        self.assertEqual(result["repeated_exact_serialized_cheat_subtree_occurrences"], 3)
        self.assertFalse(result["individual_game_labels_cheat_text_xml_or_fingerprints_exported"])

    def test_invalid_paths_or_giant_members_rejected(self):
        with self.assertRaises(ValueError):
            audit_zip(zip_bytes([("../escape.xml", b'<root/>')]))
        with self.assertRaises(ValueError):
            audit_zip(zip_bytes([("big.xml", b"a" * 10_000_001)]))

    def test_rejects_entities_and_reports_unparsed(self):
        result = audit_zip(zip_bytes([
            ("entities.xml", b'<!DOCTYPE xml [<!ENTITY a "bad">]><xml>&a;</xml>'),
            ("broken.xml", b"<notxml"),
            ("readme.txt", b"notes"),
        ]))
        self.assertEqual(result["xml_members_rejected_for_doctype_or_entity"], 1)
        self.assertEqual(result["xml_parse_error_members"], 1)
        self.assertEqual(result["non_xml_member_occurrences"], 1)

    def test_xml_local_names(self):
        self.assertEqual(local_name("{ns}cheat"), "cheat")


if __name__ == "__main__":
    unittest.main()
