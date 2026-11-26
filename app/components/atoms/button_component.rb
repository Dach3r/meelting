module Atoms
  class ButtonComponent < ViewComponent::Base
    def initialize(href:, method: :post, **html_options)
      @href = href
      @method = method
      @html_options = html_options
    end

    def call
      helpers.button_to content, @href, method: @method, class: "text-sm font-medium text-gray-600 hover:text-gray-900 cursor-pointer", **@html_options
    end
  end
end
