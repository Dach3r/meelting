import { Controller } from "@hotwired/stimulus";

export default class extends Controller {
  static targets = ["label"];

  pick(event) {
    this.labelTarget.textContent = event.currentTarget.textContent.trim();
    if (event.detail > 0) this.close();
  }

  confirm(event) {
    event.preventDefault();
    this.close();
  }

  close() {
    this.element.open = false;
    this.element.querySelector("summary").focus();
  }
}
