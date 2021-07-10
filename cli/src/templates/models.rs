pub struct {{ name }} {
{%- for field in fields %}
    pub {{ field.name }}: String,
{%- endfor %}
}