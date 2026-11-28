import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static targets = ["step", "bar", "progress", "position", "label", "submit", "back"]
  static values = { step: Number, nextLabel: String, submitLabel: String }

  stepValueChanged() {
    const current = this.stepValue
    const total = this.stepTargets.length

    this.stepTargets.forEach((step, index) => { step.hidden = index + 1 !== current })
    this.barTargets.forEach((bar, index) => {
      bar.classList.toggle("bg-forest", index < current)
      bar.classList.toggle("bg-line", index >= current)
    })
    this.progressTarget.setAttribute("aria-valuenow", current)
    this.positionTarget.textContent = `${current}/${total}`
    this.labelTarget.textContent = this.stepTargets[current - 1].dataset.label
    this.submitTarget.textContent = current === total ? this.submitLabelValue : this.nextLabelValue
    this.backTarget.hidden = current === 1
  }

  next(event) {
    if (this.stepValue >= this.stepTargets.length) return

    event.preventDefault()
    if (!this.#currentStepValid()) return

    this.stepValue++
    this.#focusCurrentStep()
  }

  back() {
    if (this.stepValue <= 1) return

    this.stepValue--
    this.#focusCurrentStep()
  }

  #currentStepValid() {
    const fields = this.stepTargets[this.stepValue - 1].querySelectorAll("input, select, textarea")
    return [...fields].every((field) => field.reportValidity())
  }

  #focusCurrentStep() {
    this.stepTargets[this.stepValue - 1].querySelector("input, select, textarea")?.focus()
  }
}
