/**
 * Client-side Form Validation for Janitor
 * Provides form validation before submission
 */

(function(window, document, $) {
    'use strict';

    // Ensure Janitor namespace exists
    window.Janitor = window.Janitor || {};

    /**
     * Validation Module
     */
    Janitor.Validation = (function() {
        
        // Validation rules
        var rules = {
            required: function(value) {
                return value !== null && value !== undefined && value.toString().trim() !== '';
            },
            email: function(value) {
                var emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
                return emailRegex.test(value);
            },
            url: function(value) {
                try {
                    new URL(value);
                    return true;
                } catch (e) {
                    return false;
                }
            },
            minLength: function(value, length) {
                return value.toString().length >= length;
            },
            maxLength: function(value, length) {
                return value.toString().length <= length;
            },
            pattern: function(value, pattern) {
                var regex = new RegExp(pattern);
                return regex.test(value);
            },
            packageName: function(value) {
                // Debian package name validation
                var packageRegex = /^[a-z0-9][a-z0-9+.-]*$/;
                return packageRegex.test(value);
            },
            vcsUrl: function(value) {
                // VCS URL validation (git, bzr, svn, etc.)
                var vcsRegex = /^(git|bzr|svn|hg|https?):\/\/.+/;
                return vcsRegex.test(value);
            }
        };

        // Error messages
        var messages = {
            required: 'This field is required',
            email: 'Please enter a valid email address',
            url: 'Please enter a valid URL',
            minLength: 'Must be at least {0} characters long',
            maxLength: 'Must be no more than {0} characters long',
            pattern: 'Invalid format',
            packageName: 'Invalid package name format',
            vcsUrl: 'Invalid VCS URL format'
        };

        /**
         * Initialize validation for all forms
         */
        function init() {
            // Attach validation to forms with validation attributes
            $('form[data-validate="true"]').each(function() {
                attachFormValidation(this);
            });

            // Real-time validation for inputs with validation attributes
            $(document).on('blur', 'input[data-validate], textarea[data-validate], select[data-validate]', function() {
                validateField(this);
            });

            // Clear validation on input
            $(document).on('input', 'input[data-validate], textarea[data-validate]', function() {
                clearFieldError(this);
            });
        }

        /**
         * Attach validation to a specific form
         */
        function attachFormValidation(form) {
            $(form).on('submit', function(e) {
                if (!validateForm(form)) {
                    e.preventDefault();
                    return false;
                }
            });
        }

        /**
         * Validate entire form
         */
        function validateForm(form) {
            var $form = $(form);
            var isValid = true;

            // Clear previous errors
            $form.find('.error-message').remove();
            $form.find('.form-control.error').removeClass('error');

            // Validate each field
            $form.find('input[data-validate], textarea[data-validate], select[data-validate]').each(function() {
                if (!validateField(this)) {
                    isValid = false;
                }
            });

            // Show summary if there are errors
            if (!isValid) {
                showFormErrors($form);
            }

            return isValid;
        }

        /**
         * Validate a single field
         */
        function validateField(field) {
            var $field = $(field);
            var value = $field.val();
            var validationRules = $field.data('validate').split('|');
            var isValid = true;
            var errorMessage = '';

            // Clear previous error
            clearFieldError(field);

            // Check each validation rule
            for (var i = 0; i < validationRules.length; i++) {
                var ruleStr = validationRules[i].trim();
                var ruleParts = ruleStr.split(':');
                var ruleName = ruleParts[0];
                var ruleParam = ruleParts[1];

                if (rules[ruleName]) {
                    var result;
                    if (ruleParam) {
                        result = rules[ruleName](value, ruleParam);
                    } else {
                        result = rules[ruleName](value);
                    }

                    if (!result) {
                        isValid = false;
                        errorMessage = getErrorMessage(ruleName, ruleParam);
                        break;
                    }
                }
            }

            if (!isValid) {
                showFieldError(field, errorMessage);
            }

            return isValid;
        }

        /**
         * Show error for a specific field
         */
        function showFieldError(field, message) {
            var $field = $(field);
            var $group = $field.closest('.form-group, .input-group');
            
            $field.addClass('error');
            
            var $error = $('<div class="error-message text-danger">')
                .text(message)
                .css('font-size', '0.875em');
            
            if ($group.length > 0) {
                $group.append($error);
            } else {
                $field.after($error);
            }
        }

        /**
         * Clear error for a specific field
         */
        function clearFieldError(field) {
            var $field = $(field);
            var $group = $field.closest('.form-group, .input-group');
            
            $field.removeClass('error');
            $group.find('.error-message').remove();
        }

        /**
         * Show form-level error summary
         */
        function showFormErrors($form) {
            var errorCount = $form.find('.error-message').length;
            
            if (errorCount > 0) {
                var $summary = $('<div class="alert alert-danger form-errors">')
                    .html('<strong>Please correct the following errors:</strong>')
                    .css('margin-bottom', '20px');
                
                $form.prepend($summary);
                
                // Scroll to first error
                var $firstError = $form.find('.form-control.error').first();
                if ($firstError.length > 0) {
                    $firstError.focus();
                    $('html, body').animate({
                        scrollTop: $firstError.offset().top - 100
                    }, 300);
                }
            }
        }

        /**
         * Get error message for a rule
         */
        function getErrorMessage(ruleName, param) {
            var message = messages[ruleName] || 'Invalid value';
            
            if (param && message.includes('{0}')) {
                message = message.replace('{0}', param);
            }
            
            return message;
        }

        /**
         * Add custom validation rule
         */
        function addRule(name, validator, message) {
            rules[name] = validator;
            if (message) {
                messages[name] = message;
            }
        }

        /**
         * Validate specific value against rules
         */
        function validate(value, ruleString) {
            var validationRules = ruleString.split('|');
            
            for (var i = 0; i < validationRules.length; i++) {
                var ruleStr = validationRules[i].trim();
                var ruleParts = ruleStr.split(':');
                var ruleName = ruleParts[0];
                var ruleParam = ruleParts[1];

                if (rules[ruleName]) {
                    var result;
                    if (ruleParam) {
                        result = rules[ruleName](value, ruleParam);
                    } else {
                        result = rules[ruleName](value);
                    }

                    if (!result) {
                        return {
                            valid: false,
                            message: getErrorMessage(ruleName, ruleParam)
                        };
                    }
                }
            }
            
            return {valid: true};
        }

        /**
         * Setup search form validation
         */
        function setupSearchValidation() {
            $('.search-form input[type="search"]').attr('data-validate', 'required|minLength:2');
            $('.package-search-input').attr('data-validate', 'packageName');
            $('.vcs-url-input').attr('data-validate', 'vcsUrl');
        }

        /**
         * Setup review form validation
         */
        function setupReviewValidation() {
            $('#review-form textarea[name="comment"]').attr('data-validate', 'maxLength:1000');
        }

        /**
         * Setup admin form validation
         */
        function setupAdminValidation() {
            $('.reschedule-form input[name="priority"]').attr('data-validate', 'pattern:^[0-9]+$');
        }

        // Initialize when DOM is ready
        $(document).ready(function() {
            init();
            setupSearchValidation();
            setupReviewValidation();
            setupAdminValidation();
        });

        // Public API
        return {
            init: init,
            validateForm: validateForm,
            validateField: validateField,
            validate: validate,
            addRule: addRule,
            rules: rules,
            messages: messages
        };
    })();

})(window, document, jQuery);