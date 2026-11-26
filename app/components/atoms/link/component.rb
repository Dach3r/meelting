module Atoms
  module Link
    class Component < ViewComponent::Base
      def initialize(href:, **html_options)
        @href = href
        @html_options = html_options
      end

      def call
        link_to content, @href, class: "text-sm font-medium text-gray-600 hover:text-gray-900", **@html_options
      end
    end
  end
end
