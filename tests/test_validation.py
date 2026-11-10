import unittest
from core.validation import validate_bucket_index

class TestBucketValidation(unittest.TestCase):
    def test_valid_boundaries(self):
        # 0 is the lower boundary
        self.assertEqual(validate_bucket_index(0), 0)
        # 19 is the upper boundary
        self.assertEqual(validate_bucket_index(19), 19)
        # Mid-range valid cases
        self.assertEqual(validate_bucket_index(5), 5)
        self.assertEqual(validate_bucket_index(12), 12)

    def test_negative_values(self):
        # -1 boundary case
        with self.assertRaises(ValueError) as ctx:
            validate_bucket_index(-1)
        self.assertIn("must be >= 0", str(ctx.exception))

        # Arbitrary negative case
        with self.assertRaises(ValueError) as ctx:
            validate_bucket_index(-25)
        self.assertIn("must be >= 0", str(ctx.exception))

    def test_out_of_range_upper(self):
        # 20 is the first invalid upper index for a 20-bucket histogram
        with self.assertRaises(ValueError) as ctx:
            validate_bucket_index(20)
        self.assertIn("must be <= 19", str(ctx.exception))

        # 10,000,000 original cheat attack case
        with self.assertRaises(ValueError) as ctx:
            validate_bucket_index(10000000)
        self.assertIn("must be <= 19", str(ctx.exception))

    def test_type_safety(self):
        # Must reject non-integers and booleans
        with self.assertRaises(TypeError):
            validate_bucket_index("5")  # string

        with self.assertRaises(TypeError):
            validate_bucket_index(5.0)  # float

        with self.assertRaises(TypeError):
            validate_bucket_index(True)  # boolean (subclass of int in Python)

if __name__ == "__main__":
    unittest.main()
