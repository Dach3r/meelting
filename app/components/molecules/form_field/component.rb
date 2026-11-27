module Molecules
  module FormField
    class Component < ViewComponent::Base
      renders_one :label_aside

      INPUTS = {
        text: :text_field,
        email: :email_field,
        password: :password_field
      }.freeze

      def initialize(form:, attribute:, label:, icon:, type: :text, **input_options)
        @form = form
        @attribute = attribute
        @label = label
        @icon = icon
        @input = INPUTS.fetch(type)
        @input_options = input_options
      end

      private
      attr_reader :form, :attribute, :label, :icon, :input, :input_options
    end
  end
end
