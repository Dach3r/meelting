import { Controller } from "@hotwired/stimulus";

const SLUG_FORMAT = /^[a-z0-9][a-z0-9._-]*$/;

export default class extends Controller {
  static targets = ["name", "url"];
  static values = { urlTemplate: String, takenSlugs: Array };

  connect() {
    this.refresh();
  }

  refresh() {
    const slug = this.#slugFor(this.nameTarget.value);
    const valid = SLUG_FORMAT.test(slug) && !this.takenSlugsValue.includes(slug);

    this.urlTarget.hidden = this.nameTarget.value.trim() === "";
    this.urlTarget.textContent = this.urlTemplateValue.replace("__slug__", slug);
    this.urlTarget.classList.toggle("text-lime-deep", valid);
    this.urlTarget.classList.toggle("text-danger", !valid);
  }

  #slugFor(name) {
    return name
      .normalize("NFD")
      .replace(/[̀-ͯ]/g, "")
      .replace(/[^a-z0-9_-]+/gi, "-")
      .replace(/-{2,}/g, "-")
      .replace(/^-|-$/g, "")
      .toLowerCase()
      .slice(0, 100)
      .replace(/-$/, "");
  }
}
