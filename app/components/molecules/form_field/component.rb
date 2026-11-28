module Molecules
  module FormField
    class Component < ViewComponent::Base
      renders_one :label_aside
      renders_one :status
      renders_one :control

      INPUTS = {
        text: :text_field,
        email: :email_field,
        password: :password_field,
        text_area: :text_area
      }.freeze

      def initialize(form:, attribute:, label:, icon:, type: :text, compact: false, **input_options)
        @form = form
        @attribute = attribute
        @label = label
        @icon = icon
        @input = INPUTS.fetch(type)
        @compact = compact
        @input_options = input_options
      end

      private
      attr_reader :form, :attribute, :label, :icon, :input, :compact, :input_options

      def shell_classes
        class_names("nb-field-shell", "nb-field-shell-area" => input == :text_area, "nb-field-shell-compact" => compact)
      end

      def input_tag
        form.public_send(input, attribute, **{ required: true, autocomplete: "off", class: "nb-field-shell-input" }.merge(input_options))
      end
    end
  end
end
