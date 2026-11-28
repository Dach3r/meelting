import { Controller } from "@hotwired/stimulus";

export default class extends Controller {
  static targets = ["menu", "menuButton"];
  static values = { cookie: String };

  menu() {
    const open = this.menuTarget.toggleAttribute("data-open");

    this.menuButtonTarget.setAttribute("aria-expanded", open);
  }

  expand() {
    this.#width(true);
  }

  collapse() {
    this.#width(false);
  }

  #width(expanded) {
    document.body.toggleAttribute("data-sidebar-expanded", expanded);
    document.cookie = `${this.cookieValue}=${expanded ? 1 : 0}; path=/; max-age=31536000; samesite=lax`;
  }
}
