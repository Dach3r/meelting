import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  static values = { url: String, frame: String, delay: { type: Number, default: 300 } }

  connect() {
    this.frame.addEventListener("turbo:frame-load", this.#showResult)
  }

  disconnect() {
    clearTimeout(this.timer)
    this.frame.removeEventListener("turbo:frame-load", this.#showResult)
  }

  check() {
    const value = this.element.value.toLowerCase().replace(/[^a-z0-9_]/g, "").replace(/^[^a-z]+/, "")
    if (value !== this.element.value) this.element.value = value

    clearTimeout(this.timer)
    this.#clearResult()
    this.timer = setTimeout(() => this.#load(value), this.delayValue)
  }

  get frame() {
    return document.getElementById(this.frameValue)
  }

  #load(value) {
    if (!value) {
      this.frame.removeAttribute("src")
      return
    }

    const src = `${this.urlValue}?${new URLSearchParams({ username: value })}`
    if (this.frame.getAttribute("src") === src) this.frame.reload()
    else this.frame.src = src
  }

  #showResult = () => {
    const availability = this.frame.querySelector("[data-availability]")?.dataset.availability
    if (!availability) return

    this.element.dataset.availability = availability
    this.element.setAttribute("aria-invalid", availability === "unavailable")
  }

  #clearResult() {
    delete this.element.dataset.availability
    this.element.removeAttribute("aria-invalid")
    this.frame.replaceChildren()
  }
}
