import { Controller } from "@hotwired/stimulus";

export default class extends Controller {
  connect() {
    this.element.addEventListener("cancel", (event) => event.preventDefault());
    this.element.addEventListener("close", () => this.element.remove(), { once: true });
    this.element.showModal();
  }

  close() {
    this.element.close();
  }
}
