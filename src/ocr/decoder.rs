#[derive(Debug, Clone)]
pub struct DecodedText {
    pub text: String,
    pub confidence: f32,
}

pub trait RecognitionDecoder {
    fn decode_logits(
        &self,
        logits: &[Vec<f32>],
        charset: &[String],
        blank_index: usize,
    ) -> anyhow::Result<DecodedText>;

    fn decode_indices(
        &self,
        indices: &[usize],
        charset: &[String],
        blank_index: usize,
    ) -> anyhow::Result<DecodedText>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CtcGreedyDecoder;

impl RecognitionDecoder for CtcGreedyDecoder {
    fn decode_logits(
        &self,
        logits: &[Vec<f32>],
        charset: &[String],
        blank_index: usize,
    ) -> anyhow::Result<DecodedText> {
        let mut indices = Vec::with_capacity(logits.len());
        let mut confidence = 1.0_f32;
        for step in logits {
            if step.is_empty() {
                continue;
            }
            let (best_idx, best_prob) = step
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(idx, prob)| (idx, *prob))
                .unwrap_or((blank_index, 0.0));
            indices.push(best_idx);
            confidence = confidence.min(best_prob.max(0.0));
        }

        let mut decoded = ctc_greedy_decode_indices(&indices, charset, blank_index);
        decoded.confidence = confidence;
        Ok(decoded)
    }

    fn decode_indices(
        &self,
        indices: &[usize],
        charset: &[String],
        blank_index: usize,
    ) -> anyhow::Result<DecodedText> {
        Ok(ctc_greedy_decode_indices(indices, charset, blank_index))
    }
}

pub fn ctc_greedy_decode_indices(
    indices: &[usize],
    charset: &[String],
    blank_index: usize,
) -> DecodedText {
    let mut out = String::new();
    let mut prev: Option<usize> = None;

    for &idx in indices {
        if Some(idx) == prev {
            continue;
        }
        prev = Some(idx);
        if idx == blank_index {
            continue;
        }
        if let Some(symbol) = charset.get(idx) {
            out.push_str(symbol);
        }
    }

    DecodedText {
        text: out,
        confidence: 1.0,
    }
}
