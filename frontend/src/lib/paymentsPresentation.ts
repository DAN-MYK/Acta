import {
  getPaymentDirectionLabel,
  PAYMENT_PREVIEW_COPY,
  PAYMENT_SCREEN_COPY,
  resolveDocumentKindMeta
} from "./config/ui";
import type { PaymentMatchCandidateDto, PaymentMatchPreviewDto } from "./types";

export function getPaymentStateLabel(matchedDoc: string): string {
  return matchedDoc ? PAYMENT_SCREEN_COPY.stateMatched(matchedDoc) : PAYMENT_SCREEN_COPY.stateUnmatched;
}

export function getPaymentDocumentKindLabel(kind: PaymentMatchCandidateDto["documentKind"]): string {
  return resolveDocumentKindMeta(kind).label;
}

export function getPaymentPreviewCopy(
  preview: PaymentMatchPreviewDto | null
): { title: string; description: string } | null {
  if (!preview) {
    return null;
  }

  return PAYMENT_PREVIEW_COPY[preview.decisionKind];
}

export function getPaymentCandidateHint(candidate: PaymentMatchCandidateDto): string {
  const hints: string[] = [];

  if (candidate.sameIban) {
    hints.push("той самий IBAN");
  }

  if (candidate.referenceHit) {
    hints.push("є збіг по призначенню");
  }

  if (candidate.textHits > 0) {
    hints.push(`текстових збігів: ${candidate.textHits}`);
  }

  hints.push(`відхилення по даті: ${candidate.daysDistance} дн.`);
  return hints.join(" • ");
}

export { getPaymentDirectionLabel };
