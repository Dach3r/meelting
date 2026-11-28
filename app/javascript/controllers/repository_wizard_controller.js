import { Controller } from "@hotwired/stimulus";

const NAV_ITEM_CURRENT = ["bg-cream"];
const NAV_DOT_CURRENT = ["border-forest", "bg-forest", "text-lime"];
const SLUG_FORMAT = /^[a-z0-9][a-z0-9._-]*$/;
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
    "url",
    "visibility",
    "license",
  ];
  static values = {
    step: Number,
    nextLabel: String,
    submitLabel: String,
    positionLabel: String,
    urlTemplate: String,
    takenSlugs: Array,
  };

  connect() {
    this.refresh();
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

  refresh() {
    const [repository, visibility] = this.summaryTargets;
    const checked = this.visibilityTargets.find((radio) => radio.checked);
    const license = this.licenseTarget.selectedOptions[0];

    this.#show(repository, this.nameTarget.value);
    this.#show(visibility, [
      checked?.closest("label").querySelector("[data-visibility-label]").textContent.trim(),
      license?.text,
    ].filter(Boolean).join(" · "));
    this.#showUrl();
  }

  #showUrl() {
    const slug = this.#slugFor(this.nameTarget.value);
    const valid = SLUG_FORMAT.test(slug) && !this.takenSlugsValue.includes(slug);

    this.urlTarget.hidden = this.nameTarget.value.trim() === "";
    this.urlTarget.textContent = this.urlTemplateValue.replace("__slug__", slug);
    this.urlTarget.classList.toggle("text-forest", valid);
    this.urlTarget.classList.toggle("text-danger", !valid);
  }

  #slugFor(name) {
    return name
      .normalize("NFD")
      .replace(/[\u0300-\u036f]/g, "")
      .replace(/[^a-z0-9_-]+/gi, "-")
      .replace(/-{2,}/g, "-")
      .replace(/^-|-$/g, "")
      .toLowerCase()
      .slice(0, 100)
      .replace(/-$/, "");
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
