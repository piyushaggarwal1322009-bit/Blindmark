"""
Validation logic for benchmark measurements and bucket indices.
"""

def validate_bucket_index(bucket: int, min_bucket: int = 0, max_bucket: int = 19) -> int:
    """
    Validates that a histogram measurement bucket index is within the allowable range [min_bucket, max_bucket].
    
    Rules:
    - Must be an integer.
    - Must be >= min_bucket (default 0; rejects negative values like -1).
    - Must be <= max_bucket (default 19; rejects values >= 20, including out-of-range scalars like 10,000,000).
    
    Raises:
        TypeError: If bucket is not an integer.
        ValueError: If bucket is negative or exceeds max_bucket.
    
    Returns:
        The validated integer bucket index.
    """
    if not isinstance(bucket, int) or isinstance(bucket, bool):
        raise TypeError(f"Bucket index must be an integer, got {type(bucket).__name__}")
    
    if bucket < min_bucket:
        raise ValueError(f"Bucket index {bucket} is invalid: must be >= {min_bucket}")
    
    if bucket > max_bucket:
        raise ValueError(f"Bucket index {bucket} is invalid: must be <= {max_bucket}")
    
    return bucket
