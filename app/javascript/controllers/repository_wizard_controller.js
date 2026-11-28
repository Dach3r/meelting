import { Controller } from "@hotwired/stimulus";

const NAV_ITEM_CURRENT = ["bg-cream"];
const NAV_DOT_CURRENT = ["border-forest", "bg-forest", "text-lime"];
const NAV_DOT_IDLE = ["border-line", "text-ink"];

export default class extends Controller {
  static targets = [
    "step",
    "navItem",
    "navDot",
    "summary",
    "position",
    "label",
    "submit",
    "submitLabel",
    "back",
    "name",
    "slug",
    "visibility",
    "license",
    "branch",
  ];
  static values = {
    step: Number,
    nextLabel: String,
    submitLabel: String,
    positionLabel: String,
  };

  connect() {
    this.slugTouched = this.slugTarget.value !== "";
  }

  stepValueChanged() {
    const current = this.stepValue;
    const total = this.stepTargets.length;

    this.stepTargets.forEach((step, index) => {
      step.hidden = index + 1 !== current;
    });
    this.navItemTargets.forEach((item, index) => {
      const active = index + 1 === current;
      item.classList.toggle(NAV_ITEM_CURRENT[0], active);
      if (active) item.setAttribute("aria-current", "step");
      else item.removeAttribute("aria-current");
      NAV_DOT_CURRENT.forEach((c) => this.navDotTargets[index].classList.toggle(c, active));
      NAV_DOT_IDLE.forEach((c) => this.navDotTargets[index].classList.toggle(c, !active));
    });
    this.positionTarget.textContent = this.positionLabelValue
      .replace("%{current}", current)
      .replace("%{total}", total);
    this.labelTarget.textContent = this.stepTargets[current - 1].dataset.label;
    this.submitLabelTarget.textContent =
      current === total ? this.submitLabelValue : this.nextLabelValue;
    this.backTarget.hidden = current === 1;
  }

  next(event) {
    if (this.stepValue >= this.stepTargets.length) return;

    event.preventDefault();
    if (!this.#currentStepValid()) return;

    this.stepValue++;
    this.#focusCurrentStep();
  }

  back() {
    if (this.stepValue <= 1) return;

    this.stepValue--;
    this.#focusCurrentStep();
  }

  suggestSlug() {
    if (this.slugTouched) return;

    this.slugTarget.value = this.nameTarget.value
      .normalize("NFD")
      .replace(/[̀-ͯ]/g, "")
      .toLowerCase()
      .replace(/[^a-z0-9._-]+/g, "-")
      .replace(/^[^a-z0-9]+|-+$/g, "");
  }

  touchSlug() {
    this.slugTouched = this.slugTarget.value !== "";
  }

  refresh() {
    const [repository, visibility, branch] = this.summaryTargets;
    const checked = this.visibilityTargets.find((radio) => radio.checked);
    const license = this.licenseTarget.selectedOptions[0];

    this.#show(repository, this.nameTarget.value);
    this.#show(visibility, [
      checked?.closest("label").querySelector("[data-visibility-label]").textContent.trim(),
      license?.text,
    ].filter(Boolean).join(" · "));
    this.#show(branch, this.branchTarget.value);
  }

  #show(element, value) {
    element.textContent = value.trim() || element.dataset.default;
  }

  #currentStepValid() {
    const fields = this.stepTargets[this.stepValue - 1].querySelectorAll(
      "input, select, textarea",
    );
    return [...fields].every((field) => field.reportValidity());
  }

  #focusCurrentStep() {
    this.stepTargets[this.stepValue - 1]
      .querySelector("input, select, textarea")
      ?.focus();
  }
}
