import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static values = { text: String, copied: String }
  static targets = ["label"]

  async copy() {
    try {
      await navigator.clipboard.writeText(this.textValue)
    } catch {
      return
    }

    const original = this.labelTarget.textContent
    this.labelTarget.textContent = this.copiedValue
    setTimeout(() => { this.labelTarget.textContent = original }, 1500)
  }
}
