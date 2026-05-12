from __future__ import annotations

from typing import Literal

OutputFormat = Literal["same", "png", "jpg", "jpeg", "gif", "webp"]


class Captioner:
    """Captioner that reuses the font system and glyph cache.

    Reuse one instance when captioning many images. Inputs and outputs are
    always bytes; callers own file IO.
    """

    def __init__(self) -> None:
        """Create a captioner with the bundled fonts."""
        ...

    @staticmethod
    def with_font(font_data: bytes) -> "Captioner":
        """Create a captioner with an additional font.

        Args:
            font_data: TrueType or OpenType font bytes.

        Returns:
            A captioner that searches the supplied font before bundled fonts.
        """
        ...

    def caption_bytes(
        self,
        data: bytes,
        text: str,
        output_format: OutputFormat = "same",
    ) -> bytes:
        """Caption an image or GIF.

        Args:
            data: Input image bytes.
            text: Caption text.
            output_format: Output encoding. ``"same"`` preserves the input
                format, except GIFs can be explicitly encoded as WebP.

        Returns:
            Captioned image bytes.

        Raises:
            ValueError: The format is unsupported or the image cannot be decoded.
        """
        ...
