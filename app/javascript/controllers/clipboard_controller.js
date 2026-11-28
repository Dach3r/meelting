import { Controller } from "@hotwired/stimulus";

export default class extends Controller {
  static targets = ["source", "label"];
  static values = { copied: String };

  async copy() {
    try {
      await navigator.clipboard.writeText(this.sourceTarget.textContent.trim());
    } catch {
      return;
    }
    const original = this.labelTarget.textContent;
    this.labelTarget.textContent = this.copiedValue;
    setTimeout(() => (this.labelTarget.textContent = original), 2000);
  }
}
