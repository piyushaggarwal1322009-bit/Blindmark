from abc import ABC, abstractmethod
from typing import List, Tuple, Any, Dict
from pydantic import BaseModel

class ClientReport(BaseModel):
    """
    Public share and encrypted input shares prepared by client.
    Raw measurements are NEVER accepted here.
    """
    nonce: str
    public_share: str
    encrypted_share_a: str
    encrypted_share_b: str

class VdafAggregatorInterface(ABC):
    """
    Interface for the future Distributed Aggregation Protocol (DAP) / VDAF engine.
    Production systems must delegate this to Rust libprio-rs services.
    """

    @abstractmethod
    def verify_and_prep(self, report: ClientReport) -> bool:
        """
        Executes zero-knowledge FLP proof verification between Leader & Helper aggregators.
        Returns True if the submission is a valid one-hot vector within bounds.
        """
        pass

    @abstractmethod
    def get_aggregate_share(self, campaign_id: str) -> List[int]:
        """
        Returns the aggregator's accumulated aggregate share vector.
        """
        pass

    @abstractmethod
    def unshard_results(self, agg_share_a: List[int], agg_share_b: List[int], num_measurements: int) -> List[int]:
        """
        Combines aggregate shares from both non-colluding aggregators to reconstruct the final histogram.
        """
        pass

