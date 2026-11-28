module Atoms
  module Chip
    class Component < ViewComponent::Base
      def initialize(**html_options)
        @html_options = html_options
      end

      def call
        tag.span content, **@html_options.except(:class), class: class_names("nb-chip", @html_options[:class])
      end
    end
  end
end
