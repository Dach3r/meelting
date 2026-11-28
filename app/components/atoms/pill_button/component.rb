module Atoms
  module PillButton
    class Component < ViewComponent::Base
      def initialize(variant: :secondary, size: :sm, **html_options)
        @variant = variant
        @size = size
        @extra_class = html_options.delete(:class)
        @html_options = html_options.reverse_merge(type: :button)
      end

      def call
        tag.button content, **@html_options, class: class_names(
          Atoms::LinkButton::Component::BASE,
          Atoms::LinkButton::Component::VARIANTS.fetch(@variant),
          Atoms::LinkButton::Component::SIZES.fetch(@size),
          @extra_class
        )
      end
    end
  end
end
