import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

const STORAGE_KEY = "doing_now.text";
const ONBOARDED_KEY = "doing_now.onboarded";
const EMPHASIS_DURATION_MS = 2000;
const MAX_LENGTH = 20;

const bubble = document.querySelector<HTMLDivElement>("#bubble");
const label = document.querySelector<HTMLSpanElement>("#label");
const input = document.querySelector<HTMLInputElement>("#input");
const counter = document.querySelector<HTMLSpanElement>("#counter");

if (!bubble || !label || !input || !counter) {
  throw new Error("UI root elements are missing.");
}

let emphasisTimerId: number | null = null;
let isComposing = false;

// A1 – Onboarding helpers
const isOnboarded = (): boolean => localStorage.getItem(ONBOARDED_KEY) === "1";

const markOnboarded = (): void => {
  localStorage.setItem(ONBOARDED_KEY, "1");
};

const getEmptyLabel = (): string =>
  isOnboarded() ? "Now" : "⌥⌘Space to set task";

// D1 – Async file-based storage
const readStoredText = async (): Promise<string> => {
  try {
    const text = await invoke<string>("read_text");
    return text.slice(0, MAX_LENGTH);
  } catch {
    // File not found on first launch – attempt localStorage migration
    const migrated = (localStorage.getItem(STORAGE_KEY) ?? "").slice(
      0,
      MAX_LENGTH,
    );
    if (migrated.length > 0) {
      try {
        await invoke("write_text", { text: migrated });
      } catch (writeError) {
        console.error("failed to migrate text to file storage", writeError);
      }
      localStorage.removeItem(STORAGE_KEY);
    }
    return migrated;
  }
};

const setText = async (rawText: string): Promise<void> => {
  const text = rawText.slice(0, MAX_LENGTH);
  try {
    await invoke("write_text", { text });
  } catch (error) {
    console.error("failed to write text", error);
  }
  label.textContent = text.length > 0 ? text : getEmptyLabel();
};

const setClickThrough = async (enable: boolean): Promise<void> => {
  try {
    await invoke("set_click_through", { enable });
  } catch (error) {
    console.error("failed to set click-through state", error);
  }
};

const clearEmphasisTimer = (): void => {
  if (emphasisTimerId !== null) {
    window.clearTimeout(emphasisTimerId);
    emphasisTimerId = null;
  }
};

// C1 – Save animation
const triggerSaveAnimation = (): void => {
  bubble.classList.remove("saving");
  void bubble.offsetWidth; // force reflow to restart animation
  bubble.classList.add("saving");
  window.setTimeout(() => {
    bubble.classList.remove("saving");
  }, 600);
};

// B2 – Update character counter
const updateCounter = (): void => {
  counter.textContent = String(MAX_LENGTH - input.value.length);
};

const enterNormalMode = (): void => {
  clearEmphasisTimer();
  bubble.classList.remove("emphasis", "edit"); // C2
  input.classList.add("hidden");
  counter.classList.add("hidden"); // B2
  label.classList.remove("hidden");
  void setClickThrough(true);
};

const enterEmphasisMode = (): void => {
  clearEmphasisTimer();
  bubble.classList.remove("edit"); // C2
  input.classList.add("hidden");
  counter.classList.add("hidden"); // B2
  label.classList.remove("hidden");
  void setClickThrough(true);
  bubble.classList.add("emphasis");
  emphasisTimerId = window.setTimeout(() => {
    bubble.classList.remove("emphasis");
    emphasisTimerId = null;
  }, EMPHASIS_DURATION_MS);
};

const enterEditMode = async (): Promise<void> => {
  clearEmphasisTimer();
  bubble.classList.add("emphasis", "edit"); // C2
  label.classList.add("hidden");
  input.classList.remove("hidden");
  counter.classList.remove("hidden"); // B2
  const storedText = await readStoredText();
  input.value = storedText;
  updateCounter(); // B2 – set initial count
  void setClickThrough(false).finally(() => {
    requestAnimationFrame(() => {
      input.focus();
      input.select();
    });
  });
};

// B2 – Live counter update
input.addEventListener("input", () => {
  updateCounter();
});

input.addEventListener("compositionstart", () => {
  isComposing = true;
});

input.addEventListener("compositionend", () => {
  isComposing = false;
});

input.addEventListener("keydown", async (event) => {
  const isImeComposing =
    event.isComposing ||
    isComposing ||
    // Safari/macOS IME fallback
    (event as KeyboardEvent & { keyCode?: number }).keyCode === 229;

  if (event.key === "Enter") {
    if (isImeComposing) {
      return;
    }
    event.preventDefault();
    markOnboarded(); // A1 – mark before setText so getEmptyLabel returns "Now"
    await setText(input.value);
    enterNormalMode();
    triggerSaveAnimation(); // C1
    return;
  }

  if (event.key === "Escape") {
    if (isImeComposing) {
      return;
    }
    enterNormalMode();
  }
});

void listen("mode", (event) => {
  const mode = String(event.payload);
  if (mode === "emphasis") {
    enterEmphasisMode();
    return;
  }
  if (mode === "edit") {
    void enterEditMode();
  }
});

// D1 + A1 – Init: read from file storage, show onboarding hint if applicable
void (async () => {
  const t = await readStoredText();
  label.textContent = t.length > 0 ? t : getEmptyLabel();
  enterNormalMode();
})();
