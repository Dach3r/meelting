import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static values = { url: String, frame: String, delay: { type: Number, default: 300 } }

  disconnect() {
    clearTimeout(this.timer)
  }

  check() {
    const value = this.element.value.toLowerCase().replace(/[^a-z0-9_]/g, "").replace(/^[^a-z]+/, "")
    if (value !== this.element.value) this.element.value = value

    clearTimeout(this.timer)
    this.timer = setTimeout(() => this.#load(value), this.delayValue)
  }

  #load(value) {
    const frame = document.getElementById(this.frameValue)
    if (!value) {
      frame.removeAttribute("src")
      frame.replaceChildren()
      return
    }
    frame.src = `${this.urlValue}?${new URLSearchParams({ username: value })}`
  }
}
