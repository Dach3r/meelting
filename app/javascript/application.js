// Configure your import map in config/importmap.rb. Read more: https://github.com/rails/importmap-rails
import "@hotwired/turbo-rails"
import "controllers"

document.addEventListener("invalid", (event) => {
  event.preventDefault()
  if (event.target === event.target.form?.querySelector(":invalid")) event.target.focus()
}, true)
