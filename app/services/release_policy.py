from abc import ABC, abstractmethod
from typing import List, Tuple, Optional
from datetime import datetime, timezone
from app.models import Campaign, AggregationResult

class DifferentialPrivacyProvider(ABC):
    """
    Interface for future differential privacy perturbation mechanism.
    Production systems must calibrate Laplace or Gaussian noise based on
    exact sensitivity and privacy budgets (epsilon, delta).
    """
    @abstractmethod
    def apply_noise(self, histogram: List[int], epsilon: float) -> List[int]:
        pass


class DevelopmentReleasePolicy:
    """
    Development-only campaign result release policy.
    
    Guarantees:
    - Never releases results if participant count < campaign.minimum_participants.
    - Masks/withholds raw histogram when minimum threshold is unmet.
    - Labels status clearly as 'withheld' or 'released'.
    - Does NOT claim formal differential privacy until an audited provider is integrated.
    """

    @staticmethod
    def evaluate_release(
        campaign: Campaign,
        histogram: List[int],
        participant_count: int,
        dp_provider: Optional[DifferentialPrivacyProvider] = None
    ) -> Tuple[str, Optional[List[int]], Optional[str]]:
        """
        Evaluates whether a campaign's aggregate histogram may be released to the public.
        
        Returns:
            (status: str, sanitized_histogram: Optional[List[int]], message: str)
        """
        # Threshold enforcement (k-anonymity gate)
        if participant_count < campaign.minimum_participants:
            return (
                "withheld",
                None,
                f"Cohort threshold not reached. Current: {participant_count}, Required: {campaign.minimum_participants}"
            )

        # Ensure histogram length matches bucket configuration
        expected_buckets = len(campaign.bucket_configuration)
        if len(histogram) != expected_buckets:
            return (
                "withheld",
                None,
                f"Histogram dimension mismatch: expected {expected_buckets} buckets, got {len(histogram)}"
            )

        # Apply differential privacy if provider is configured
        output_histogram = list(histogram)
        if dp_provider is not None:
            output_histogram = dp_provider.apply_noise(output_histogram, epsilon=1.0)

        return (
            "released",
            output_histogram,
            "Campaign results successfully verified and released under cohort threshold policy."
        )

