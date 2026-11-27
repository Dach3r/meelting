import { Controller } from "@hotwired/stimulus"

// Expands one repository at a time and remembers it on the server so the next page load
// renders the same state without a flash.
export default class extends Controller {
  static targets = ["toggle", "menu", "menuButton"]
  static values = { url: String }

  menu() {
    const open = this.menuTarget.toggleAttribute("data-open")

    this.menuButtonTarget.setAttribute("aria-expanded", open)
  }

  toggle({ currentTarget: button }) {
    const opening = button.getAttribute("aria-expanded") !== "true"

    this.toggleTargets.forEach((target) => this.#set(target, false))
    if (opening) this.#set(button, true)

    this.#persist(opening ? button.dataset.slug : "")
  }

  #set(button, open) {
    button.setAttribute("aria-expanded", open)
    document.getElementById(button.getAttribute("aria-controls")).hidden = !open
  }

  #persist(slug) {
    const token = document.querySelector("meta[name=csrf-token]")?.content

    fetch(this.urlValue, {
      method: "PATCH",
      headers: { "X-CSRF-Token": token, "Content-Type": "application/x-www-form-urlencoded" },
      body: new URLSearchParams({ repository_slug: slug }),
      keepalive: true
    })
  }
}
