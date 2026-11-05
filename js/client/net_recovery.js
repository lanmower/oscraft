import { FRAME } from "./wire/frame.js";

export function createFrameRecovery({ applyFrame, requestKeyframe, log = () => {} }) {
  let awaiting = false;
  let requested = false;
  return {
    apply(units, frame, peerKey) {
      if (awaiting && frame.type === FRAME.DELTA) return { applied: false, awaiting: true };
      const applied = applyFrame(units, frame);
      if (applied === null) {
        if (!requested) {
          requested = true;
          log(peerKey);
          requestKeyframe(peerKey);
        }
        awaiting = true;
        return { applied: false, awaiting: true };
      }
      awaiting = false;
      requested = false;
      return { applied: true, units: applied, awaiting: false };
    },
    lostPeer() {
      awaiting = true;
    },
    isAwaiting() {
      return awaiting;
    },
  };
}
