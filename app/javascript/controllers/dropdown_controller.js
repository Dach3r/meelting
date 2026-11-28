import { Controller } from "@hotwired/stimulus";

export default class extends Controller {
  close(event) {
    if (!this.element.open) return;
    if (event.type === "click" && this.element.contains(event.target)) return;

    this.element.open = false;
    if (event.type === "keydown") this.element.querySelector("summary").focus();
  }
}
