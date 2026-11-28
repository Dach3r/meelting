module Molecules
  module SelectMenu
    class Component < ViewComponent::Base
      def initialize(form:, attribute:, label:, icon:, options:)
        @form = form
        @attribute = attribute
        @label = label
        @icon = icon
        @options = options
      end

      private
      attr_reader :form, :attribute, :label, :icon, :options

      def selected_value
        form.object&.public_send(attribute).to_s
      end

      def selected_label
        options.find { |_, value| value.to_s == selected_value }&.first || options.first&.first
      end

      def checked?(value)
        value.to_s == selected_value
      end
    end
  end
end
